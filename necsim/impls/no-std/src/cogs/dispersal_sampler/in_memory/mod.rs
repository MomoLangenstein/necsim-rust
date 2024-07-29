#![allow(non_local_definitions)] // FIXME: displaydoc

use necsim_core::cogs::{DispersalSampler, Habitat, MathsCore, RngCore};
use necsim_core_bond::NonNegativeF64;

use crate::array2d::Array2D;

mod contract;

pub mod alias;
pub mod cumulative;
pub mod packed_alias;
pub mod packed_separable_alias;
pub mod separable_alias;

#[expect(clippy::module_name_repetitions)]
pub trait InMemoryDispersalSampler<M: MathsCore, H: Habitat<M>, G: RngCore<M>>:
    DispersalSampler<M, H, G> + Sized
{
    /// Creates a new in-memory dispersal sampler from the `dispersal` map and
    /// the habitat.
    ///
    /// # Errors
    ///
    /// `Err(DispersalMapSizeMismatch)` is returned iff the dimensions of
    /// `dispersal` are not `ExE` given `E=WxH` where habitat has width `W`
    /// and height `W`.
    ///
    /// `Err(DispersalToNonHabitat)` is returned iff any dispersal targets a
    /// non-habitat cell.
    ///
    /// `Err(DispersalFromNonHabitat)` is returned iff any non-habitat cell has
    /// any dispersal.
    ///
    /// `Err(NoDispersalFromHabitat)` is returned iff any habitat cell does not
    /// have any dispersal.
    fn new(
        dispersal: &Array2D<NonNegativeF64>,
        habitat: &H,
    ) -> Result<Self, InMemoryDispersalSamplerError>;
}

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, displaydoc::Display)]
pub enum InMemoryDispersalSamplerError {
    /** The size of the dispersal map is inconsistent with the size of the
    habitat. */
    DispersalMapSizeMismatch,
    /// Some dispersal targets a non-habitat cell.
    DispersalToNonHabitat,
    /// Some non-habitat cell has outgoing dispersals.
    DispersalFromNonHabitat,
    /// Some habitat cell does not have any outgoing dispersals.
    NoDispersalFromHabitat,
}
