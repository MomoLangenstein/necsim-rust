use alloc::boxed::Box;
use core::marker::PhantomData;

use necsim_core::{
    cogs::{Backup, Habitat, MathsCore},
    landscape::{LandscapeExtent, Location},
};
use necsim_core_bond::OffByOneU32;
use necsim_partitioning_core::partition::Partition;

use crate::decomposition::Decomposition;

mod area;
mod weight;

#[cfg(test)]
mod test;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct EqualDecomposition<M: MathsCore, H: Habitat<M>> {
    subdomain: Partition,

    extent: LandscapeExtent,
    morton: (u8, u8),

    indices: Box<[u64]>,

    _marker: PhantomData<(M, H)>,
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>> Backup for EqualDecomposition<M, H> {
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            subdomain: self.subdomain,
            extent: self.extent.clone(),
            morton: self.morton,
            indices: self.indices.clone(),
            _marker: PhantomData::<(M, H)>,
        }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>> Decomposition<M, H> for EqualDecomposition<M, H> {
    fn get_subdomain(&self) -> Partition {
        self.subdomain
    }

    #[debug_requires(
        habitat.get_extent() == &self.extent,
        "habitat has a matching extent"
    )]
    fn map_location_to_subdomain_rank(&self, location: &Location, habitat: &H) -> u32 {
        let dx = location.x() - self.extent.origin().x();
        let dy = location.y() - self.extent.origin().y();

        let morton_index = Self::map_x_y_to_morton(self.morton.0, self.morton.1, dx, dy);

        #[expect(clippy::cast_possible_truncation)]
        match self.indices.binary_search(&morton_index) {
            Ok(index) => (index + 1) as u32,
            Err(index) => index as u32,
        }
    }
}

impl<M: MathsCore, H: Habitat<M>> EqualDecomposition<M, H> {
    fn next_log2(coord: OffByOneU32) -> u8 {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        if coord.get() > 1 {
            M::ceil(M::ln(f64::from(coord)) / core::f64::consts::LN_2) as u8
        } else {
            0
        }
    }

    fn map_x_y_to_morton(mut morton_x: u8, mut morton_y: u8, mut dx: u32, mut dy: u32) -> u64 {
        let mut morton_index = 0_u64;

        let morton = morton_x.min(morton_y);

        for m in 0..morton {
            morton_index |= u64::from(dx & 0x1_u32) << (m * 2);
            dx >>= 1;

            morton_index |= u64::from(dy & 0x1_u32) << (m * 2 + 1);
            dy >>= 1;
        }

        morton_x -= morton;
        morton_y -= morton;

        if morton_x > 0 {
            morton_index |= u64::from(dx) << (morton * 2);
        }

        if morton_y > 0 {
            morton_index |= u64::from(dy) << (morton * 2 + morton_x);
        }

        morton_index
    }
}
