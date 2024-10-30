use crate::{
    impl_finalise, impl_report,
    reporter::{boolean::Or, Reporter},
};

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct ReporterCombinator<F: Reporter, T: Reporter> {
    front: F,
    tail: T, // R = ReporterCombinator<...>
}

impl<F: Reporter, T: Reporter> ReporterCombinator<F, T> {
    #[must_use]
    /// # Safety
    /// This constructor should not be used directly to combinate reporters.
    /// Use the `ReporterGroup![...]` macro instead.
    pub unsafe fn new(front: F, tail: T) -> Self {
        Self { front, tail }
    }

    #[must_use]
    /// # Safety
    /// This destructor should not be used directly to decompose reporters.
    /// Use the `ReporterUnGroup!{reporter => [...]}` macro instead.
    pub unsafe fn wen(self) -> (F, T) {
        (self.front, self.tail)
    }
}

impl<F: Reporter, T: Reporter> Reporter for ReporterCombinator<F, T>
where
    F::ReportSpeciation: Or<T::ReportSpeciation>,
    F::ReportDispersal: Or<T::ReportDispersal>,
    F::ReportProgress: Or<T::ReportProgress>,
{
    impl_report!(speciation(&mut self, speciation: MaybeUsed<
        <F::ReportSpeciation as Or<T::ReportSpeciation>
    >::RESULT>) {
        self.front.report_speciation(speciation.into());
        self.tail.report_speciation(speciation.into());
    });

    impl_report!(dispersal(&mut self, dispersal: MaybeUsed<
        <F::ReportDispersal as Or<T::ReportDispersal>
    >::RESULT>) {
        self.front.report_dispersal(dispersal.into());
        self.tail.report_dispersal(dispersal.into());
    });

    impl_report!(progress(&mut self, progress: MaybeUsed<
        <F::ReportProgress as Or<T::ReportProgress>
    >::RESULT>) {
        self.front.report_progress(progress.into());
        self.tail.report_progress(progress.into());
    });

    impl_finalise!((self) {
        self.front.finalise();
        self.tail.finalise();
    });

    fn initialise(&mut self) -> Result<(), alloc::string::String> {
        self.front
            .initialise()
            .and_then(|()| self.tail.initialise())
    }
}
