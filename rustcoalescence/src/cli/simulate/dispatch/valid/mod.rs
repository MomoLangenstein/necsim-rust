use necsim_core_bond::{NonNegativeF64, OpenClosedUnitF64 as PositiveUnitF64};
use necsim_impls_std::event_log::recorder::EventLogRecorder;
use necsim_plugins_core::import::AnyReporterPluginVec;

use crate::{
    args::{Algorithm, Partitioning, Sample, Scenario},
    cli::simulate::SimulationOutcome,
};

use super::super::BufferingSimulateArgsBuilder;

mod algorithm_scenario;
mod info;
mod launch;
mod partitioning;
mod rng;

#[allow(clippy::too_many_arguments)]
pub(in super::super) fn dispatch(
    partitioning: Partitioning,
    event_log: Option<EventLogRecorder>,
    reporters: AnyReporterPluginVec,

    speciation_probability_per_generation: PositiveUnitF64,
    sample: Sample,
    scenario: Scenario,
    algorithm: Algorithm,
    pause_before: Option<NonNegativeF64>,

    ron_args: &str,
    normalised_args: &mut BufferingSimulateArgsBuilder,
) -> anyhow::Result<SimulationOutcome> {
    partitioning::dispatch(
        partitioning,
        event_log,
        reporters,
        speciation_probability_per_generation,
        sample,
        scenario,
        algorithm,
        pause_before,
        ron_args,
        normalised_args,
    )
}
