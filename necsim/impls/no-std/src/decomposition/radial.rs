use libm::atan2;

use necsim_core::{
    cogs::{Backup, Habitat, MathsCore},
    landscape::Location,
};
use necsim_partitioning_core::partition::Partition;

use crate::decomposition::Decomposition;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct RadialDecomposition {
    subdomain: Partition,
}

impl RadialDecomposition {
    #[must_use]
    pub fn new(subdomain: Partition) -> Self {
        Self { subdomain }
    }
}

#[contract_trait]
impl Backup for RadialDecomposition {
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            subdomain: self.subdomain,
        }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>> Decomposition<M, H> for RadialDecomposition {
    fn get_subdomain(&self) -> Partition {
        self.subdomain
    }

    fn map_location_to_subdomain_rank(&self, location: &Location, habitat: &H) -> u32 {
        let extent = habitat.get_extent();

        let neutral_x = location.x().wrapping_sub(extent.origin().x());
        let neutral_y = location.y().wrapping_sub(extent.origin().y());

        #[expect(clippy::cast_precision_loss)]
        let fraction = (atan2(
            (i64::from(neutral_y) - i64::from(extent.height()) / 2) as f64,
            (i64::from(neutral_x) - i64::from(extent.width()) / 2) as f64,
        ) * core::f64::consts::FRAC_1_PI
            * 0.5_f64)
            + 0.5_f64;

        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            (M::floor(f64::from(self.subdomain.size().get()) * fraction) as u32)
                .min(self.subdomain.size().get() - 1)
        }
    }
}
