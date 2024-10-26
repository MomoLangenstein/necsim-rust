use std::fmt;

use necsim_core::reporter::{boolean::Boolean, FilteredReporter, Reporter};

use necsim_partitioning_core::reporter::{FinalisableReporter, ReporterContext};

use necsim_plugins_core::import::ReporterPluginVec;

pub struct DynamicReporterContext<
    ReportSpeciation: Boolean,
    ReportDispersal: Boolean,
    ReportProgress: Boolean,
> {
    reporter: ReporterPluginVec<ReportSpeciation, ReportDispersal, ReportProgress>,
}

impl<ReportSpeciation: Boolean, ReportDispersal: Boolean, ReportProgress: Boolean> fmt::Debug
    for DynamicReporterContext<ReportSpeciation, ReportDispersal, ReportProgress>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(DynamicReporterContext))
            .field("reporter", &self.reporter)
            .finish()
    }
}

impl<ReportSpeciation: Boolean, ReportDispersal: Boolean, ReportProgress: Boolean>
    DynamicReporterContext<ReportSpeciation, ReportDispersal, ReportProgress>
{
    #[allow(dead_code)] // FIXME: use expect
    pub fn new(
        reporter: ReporterPluginVec<ReportSpeciation, ReportDispersal, ReportProgress>,
    ) -> Self {
        Self { reporter }
    }
}

impl<ReportSpeciation: Boolean, ReportDispersal: Boolean, ReportProgress: Boolean> ReporterContext
    for DynamicReporterContext<ReportSpeciation, ReportDispersal, ReportProgress>
{
    type Reporter = ReporterPluginVec<ReportSpeciation, ReportDispersal, ReportProgress>;

    fn try_build<KeepSpeciation: Boolean, KeepDispersal: Boolean, KeepProgress: Boolean>(
        self,
    ) -> anyhow::Result<FilteredReporter<Self::Reporter, KeepSpeciation, KeepDispersal, KeepProgress>>
    {
        let mut filtered_reporter = self
            .reporter
            .internal_filter::<KeepSpeciation, KeepDispersal, KeepProgress>();

        match filtered_reporter.initialise() {
            Ok(()) => Ok(FilteredReporter::from(filtered_reporter)),
            Err(err) => Err(anyhow::Error::msg(err)),
        }
    }
}

#[cfg_attr(
    not(any(
        feature = "gillespie-algorithms",
        feature = "independent-algorithm",
        feature = "cuda-algorithm"
    )),
    allow(dead_code)
)]
#[expect(clippy::module_name_repetitions)]
pub enum FinalisablePartitioningReporter<R: Reporter> {
    Monolithic(<necsim_partitioning_monolithic::MonolithicPartitioning as necsim_partitioning_core::Partitioning>::FinalisableReporter<R>),
    #[cfg(feature = "mpi-partitioning")]
    Mpi(<necsim_partitioning_mpi::MpiPartitioning as necsim_partitioning_core::Partitioning>::FinalisableReporter<R>),
    #[cfg(feature = "threads-partitioning")]
    Threads(<necsim_partitioning_threads::ThreadsPartitioning as necsim_partitioning_core::Partitioning>::FinalisableReporter<R>),
}

impl<R: Reporter> FinalisableReporter for FinalisablePartitioningReporter<R> {
    fn finalise(self) {
        match self {
            Self::Monolithic(reporter) => reporter.finalise(),
            #[cfg(feature = "mpi-partitioning")]
            Self::Mpi(reporter) => reporter.finalise(),
            #[cfg(feature = "threads-partitioning")]
            Self::Threads(reporter) => reporter.finalise(),
        }
    }
}
