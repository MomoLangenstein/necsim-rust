use alloc::vec::Vec;
use core::marker::PhantomData;

use necsim_core::cogs::{Habitat, MathsCore};
use necsim_partitioning_core::partition::Partition;

use super::EqualDecomposition;

impl<M: MathsCore, H: Habitat<M>> EqualDecomposition<M, H> {
    /// # Errors
    ///
    /// Returns `Ok(Self)` iff the `habitat` can be partitioned into
    ///  `subdomain.size()` by area, otherwise returns `Err(Self)`.
    pub fn area(habitat: &H, subdomain: Partition) -> Result<Self, Self> {
        let extent = habitat.get_extent().clone();

        let mut indices = Vec::with_capacity(subdomain.size().get() as usize);

        let morton_x = Self::next_log2(extent.width());
        let morton_y = Self::next_log2(extent.height());

        for location in habitat.iter_habitable_locations() {
            indices.push(Self::map_x_y_to_morton(
                morton_x,
                morton_y,
                location.x() - extent.origin().x(),
                location.y() - extent.origin().y(),
            ));
        }

        indices.sort_unstable();

        let num_indices = indices.len() as u64;
        let mut last_rank = 0;

        let indices: Vec<u64> = indices
            .into_iter()
            .enumerate()
            .filter_map(|(i, index)| {
                #[expect(clippy::cast_possible_truncation)]
                let next_rank =
                    ((i as u64) * u64::from(subdomain.size().get()) / num_indices) as u32;

                if next_rank == last_rank {
                    None
                } else {
                    last_rank = next_rank;

                    Some(index)
                }
            })
            .collect();

        let decomposition = Self {
            subdomain,

            extent,
            morton: (morton_x, morton_y),

            indices: indices.into_boxed_slice(),

            _marker: PhantomData::<(M, H)>,
        };

        if (decomposition.indices.len() + 1) == (subdomain.size().get() as usize) {
            Ok(decomposition)
        } else {
            Err(decomposition)
        }
    }
}
