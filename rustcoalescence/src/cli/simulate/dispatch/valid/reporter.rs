use necsim_core_bond::{NonNegativeF64, OpenClosedUnitF64 as PositiveUnitF64};
use necsim_impls_std::event_log::recorder::EventLogConfig;
use necsim_plugins_core::{import::AnyReporterPluginVec, match_any_reporter_plugin_vec};

use crate::{
    args::config::{
        algorithm::Algorithm, partitioning::Partitioning, sample::Sample, scenario::Scenario,
    },
    cli::simulate::SimulationOutcome,
    reporter::DynamicReporterContext,
};

use super::{super::super::BufferingSimulateArgsBuilder, algorithm_scenario};

#[expect(clippy::too_many_arguments)]
pub(super) fn dispatch(
    partitioning: Partitioning,
    event_log: Option<EventLogConfig>,
    reporters: AnyReporterPluginVec,

    speciation_probability_per_generation: PositiveUnitF64,
    sample: Sample,
    scenario: Scenario,
    algorithm: Algorithm,
    pause_before: Option<NonNegativeF64>,

    ron_args: &str,
    normalised_args: &mut BufferingSimulateArgsBuilder,
) -> anyhow::Result<SimulationOutcome> {
    match_any_reporter_plugin_vec!(reporters => |reporter| {
        algorithm_scenario::dispatch(
            partitioning, event_log, DynamicReporterContext::new(reporter),
            speciation_probability_per_generation, sample, scenario,
            algorithm, pause_before, ron_args, normalised_args,
        )
    })
}
