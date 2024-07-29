mod extent;
mod location;

#[allow(clippy::module_name_repetitions)] // FIXME: use expect
pub use extent::{LandscapeExtent, LocationIterator};
pub use location::{IndexedLocation, Location};
