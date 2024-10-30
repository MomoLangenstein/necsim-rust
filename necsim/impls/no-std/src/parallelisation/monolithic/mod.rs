mod reporter;

pub mod averaging;
pub mod lockstep;
#[expect(clippy::module_inception)]
pub mod monolithic;
pub mod optimistic;
pub mod optimistic_lockstep;
