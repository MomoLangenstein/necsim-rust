use crate::event::{DispersalEvent, SpeciationEvent};

mod combinator;
mod filter;
mod group;
mod r#impl;
mod null;

use boolean::Boolean;
use used::MaybeUsed;

pub mod boolean;
pub mod used;

#[expect(clippy::module_name_repetitions)]
pub use combinator::ReporterCombinator;
#[expect(clippy::module_name_repetitions)]
pub use filter::FilteredReporter;
#[expect(clippy::module_name_repetitions)]
pub use null::NullReporter;

pub trait Reporter: core::fmt::Debug {
    type ReportSpeciation: Boolean;
    type ReportDispersal: Boolean;
    type ReportProgress: Boolean;

    fn report_speciation(&mut self, event: &MaybeUsed<SpeciationEvent, Self::ReportSpeciation>);

    fn report_dispersal(&mut self, event: &MaybeUsed<DispersalEvent, Self::ReportDispersal>);

    fn report_progress(&mut self, remaining: &MaybeUsed<u64, Self::ReportProgress>);

    /// This `initialise` hook can be used to commit to make final
    /// initialisation steps which have side effects.
    ///
    /// # Errors
    ///
    /// Return an error `String` iff initialisation failed.
    /// The calling code should take this as a hint to abort.
    fn initialise(&mut self) -> Result<(), alloc::string::String> {
        Ok(())
    }

    fn finalise(self)
    where
        Self: Sized,
    {
        core::mem::drop(self);
    }

    /// # Safety
    ///
    /// This method should not be implemented manually
    //  please - use the`impl_finalise` macro instead.
    unsafe fn finalise_boxed(self: alloc::boxed::Box<Self>) {
        core::mem::drop(self);
    }
}
