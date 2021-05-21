use core::marker::PhantomData;

use alloc::fmt;

use crate::{
    impl_finalise, impl_report,
    reporter::{
        boolean::{And, Boolean},
        Reporter,
    },
};

#[allow(clippy::module_name_repetitions)]
pub struct FilteredReporter<
    R: Reporter,
    KeepSpeciation: Boolean,
    KeepDispersal: Boolean,
    KeepProgress: Boolean,
> {
    reporter: R,
    marker: PhantomData<(KeepSpeciation, KeepDispersal, KeepProgress)>,
}

impl<R: Reporter, KeepSpeciation: Boolean, KeepDispersal: Boolean, KeepProgress: Boolean> fmt::Debug
    for FilteredReporter<R, KeepSpeciation, KeepDispersal, KeepProgress>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("FilteredReporter")
            .field("reporter", &self.reporter)
            .finish()
    }
}

impl<R: Reporter, KeepSpeciation: Boolean, KeepDispersal: Boolean, KeepProgress: Boolean> From<R>
    for FilteredReporter<R, KeepSpeciation, KeepDispersal, KeepProgress>
{
    fn from(reporter: R) -> Self {
        Self {
            reporter,
            marker: PhantomData,
        }
    }
}

impl<R: Reporter, KeepSpeciation: Boolean, KeepDispersal: Boolean, KeepProgress: Boolean> Reporter
    for FilteredReporter<R, KeepSpeciation, KeepDispersal, KeepProgress>
where
    KeepSpeciation: And<R::ReportSpeciation>,
    KeepDispersal: And<R::ReportDispersal>,
    KeepProgress: And<R::ReportProgress>,
{
    impl_report!(speciation(&mut self, speciation: MaybeUsed<
        <KeepSpeciation as And<R::ReportSpeciation>
    >::RESULT>) {
        self.reporter.report_speciation(speciation.into())
    });

    impl_report!(dispersal(&mut self, dispersal: MaybeUsed<
        <KeepDispersal as And<R::ReportDispersal>
    >::RESULT>) {
        self.reporter.report_dispersal(dispersal.into())
    });

    impl_report!(progress(&mut self, progress: MaybeUsed<
        <KeepProgress as And<R::ReportProgress>
    >::RESULT>) {
        self.reporter.report_progress(progress.into())
    });

    impl_finalise!((self) {
        if Self::ReportSpeciation::VALUE || Self::ReportDispersal::VALUE || Self::ReportProgress::VALUE {
            self.reporter.finalise()
        }
    });

    fn initialise(&mut self) -> Result<(), alloc::string::String> {
        if Self::ReportSpeciation::VALUE
            || Self::ReportDispersal::VALUE
            || Self::ReportProgress::VALUE
        {
            self.reporter.initialise()
        } else {
            Ok(())
        }
    }
}
