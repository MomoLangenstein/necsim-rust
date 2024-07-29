use necsim_core_bond::{NonNegativeF64, OpenClosedUnitF64 as PositiveUnitF64};
use necsim_impls_std::event_log::recorder::EventLogConfig;
use necsim_plugins_core::import::AnyReporterPluginVec;

use crate::{
    args::config::{
        algorithm::Algorithm, partitioning::Partitioning, sample::Sample, scenario::Scenario,
    },
    cli::simulate::SimulationOutcome,
};

use super::super::BufferingSimulateArgsBuilder;

#[expect(clippy::too_many_arguments, clippy::needless_pass_by_value)]
pub(in super::super) fn dispatch(
    _partitioning: Partitioning,
    _event_log: Option<EventLogConfig>,
    _reporters: AnyReporterPluginVec,

    _speciation_probability_per_generation: PositiveUnitF64,
    _sample: Sample,
    _scenario: Scenario,
    _algorithm: Algorithm,
    _pause_before: Option<NonNegativeF64>,

    _ron_args: &str,
    _normalised_args: &mut BufferingSimulateArgsBuilder,
) -> anyhow::Result<SimulationOutcome> {
    extern "C" {
        fn simulate_dispatch_without_algorithm() -> !;
    }

    unsafe { simulate_dispatch_without_algorithm() }
}
