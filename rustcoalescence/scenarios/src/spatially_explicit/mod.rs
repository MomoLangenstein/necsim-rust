mod maps;
mod turnover;

#[cfg(any(
    feature = "spatially-explicit-uniform-turnover",
    feature = "spatially-explicit-turnover-map",
))]
#[allow(clippy::module_name_repetitions)] // FIXME: use expect
pub use turnover::{SpatiallyExplicitArgumentVariants, SpatiallyExplicitArguments};

#[cfg(feature = "spatially-explicit-turnover-map")]
pub use turnover::map;

#[cfg(feature = "spatially-explicit-uniform-turnover")]
pub use turnover::uniform;
