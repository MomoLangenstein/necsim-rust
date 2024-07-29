use necsim_core::reporter::{boolean::Boolean, FilteredReporter, Reporter};

#[expect(clippy::module_name_repetitions)]
pub trait ReporterContext: core::fmt::Debug {
    type Reporter: Reporter;

    /// # Errors
    ///
    /// Return any error which occured while building the reporter(s)
    fn try_build<KeepSpeciation: Boolean, KeepDispersal: Boolean, KeepProgress: Boolean>(
        self,
    ) -> anyhow::Result<FilteredReporter<Self::Reporter, KeepSpeciation, KeepDispersal, KeepProgress>>;
}

#[expect(clippy::module_name_repetitions)]
pub trait FinalisableReporter {
    fn finalise(self);
}

#[expect(clippy::module_name_repetitions)]
pub struct OpaqueFinalisableReporter<R: Reporter> {
    reporter: R,
}

impl<R: Reporter> OpaqueFinalisableReporter<R> {
    #[must_use]
    pub fn new(reporter: R) -> Self {
        Self { reporter }
    }
}

impl<R: Reporter> From<R> for OpaqueFinalisableReporter<R> {
    fn from(reporter: R) -> Self {
        Self::new(reporter)
    }
}

impl<R: Reporter> FinalisableReporter for OpaqueFinalisableReporter<R> {
    fn finalise(self) {
        self.reporter.finalise();
    }
}
