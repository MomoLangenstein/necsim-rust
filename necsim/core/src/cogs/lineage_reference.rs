use core::hash::Hash;

use super::{Habitat, MathsCore};

pub trait LineageReference<M: MathsCore, H: Habitat<M>>:
    crate::cogs::Backup + PartialEq + Eq + Hash + core::fmt::Debug
{
}
