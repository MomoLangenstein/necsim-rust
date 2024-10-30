use necsim_core::{
    cogs::{Backup, Habitat, MathsCore},
    landscape::Location,
};
use necsim_partitioning_core::partition::Partition;

pub mod equal;
pub mod modulo;
pub mod monolithic;
pub mod radial;

#[contract_trait]
pub trait Decomposition<M: MathsCore, H: Habitat<M>>: Backup + Sized + core::fmt::Debug {
    fn get_subdomain(&self) -> Partition;

    #[debug_requires(habitat.is_location_habitable(location), "location is habitable")]
    #[debug_ensures(
        ret < self.get_subdomain().size().get(),
        "subdomain rank is in range [0, self.get_subdomain().size())"
    )]
    fn map_location_to_subdomain_rank(&self, location: &Location, habitat: &H) -> u32;
}
