use std::fmt::Write;

use anyhow::{Context, Result};

use necsim_impls_std::event_log::recorder::EventLogConfig;
use rustcoalescence_algorithms::{result::SimulationOutcome, AlgorithmDispatch};

use necsim_core::{
    cogs::{MathsCore, RngCore},
    reporter::{boolean::Boolean, Reporter},
};
use necsim_core_bond::NonNegativeF64;
use necsim_partitioning_core::reporter::{FinalisableReporter, ReporterContext};

use rustcoalescence_scenarios::{Scenario, ScenarioCogs};

use crate::args::{
    config::{
        partitioning::Partitioning,
        sample::{Sample, SampleMode, SampleModeRestart},
    },
    utils::parse::try_print,
};

use super::{super::super::BufferingSimulateArgsBuilder, partitioning};

#[expect(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn dispatch<
    M: MathsCore,
    G: RngCore<M>,
    A: AlgorithmDispatch<M, G, O, R>,
    O: Scenario<M, G>,
    R: Reporter,
    P: ReporterContext<Reporter = R>,
>(
    partitioning: Partitioning,
    event_log: Option<EventLogConfig>,
    reporter_context: P,

    sample: Sample,
    rng: G,
    scenario: ScenarioCogs<M, G, O>,
    algorithm_args: A::Arguments,
    pause_before: Option<NonNegativeF64>,

    normalised_args: &BufferingSimulateArgsBuilder,
) -> anyhow::Result<SimulationOutcome<M, G>>
where
    Result<SimulationOutcome<M, G>, A::Error>: anyhow::Context<SimulationOutcome<M, G>, A::Error>,
{
    let config_str = normalised_args
        .build()
        .map_err(anyhow::Error::new)
        .and_then(|simulate_args| try_print(&simulate_args))
        .context("Failed to normalise the simulation config.")?;

    if log::log_enabled!(log::Level::Info) {
        println!("\n{:=^80}\n", " Simulation Configuration ");
        println!("{}", config_str.trim_start_matches("Simulate"));
        println!("\n{:=^80}\n", " Simulation Configuration ");
    }

    let mut resume_pause = String::from("The simulation will ");
    match sample.mode {
        SampleMode::Genesis => resume_pause.push_str("start fresh"),
        SampleMode::Resume => resume_pause.push_str("resume"),
        SampleMode::FixUp(_) => resume_pause.push_str("fix-up for a restart"),
        SampleMode::Restart(SampleModeRestart { after, .. }) => {
            write!(resume_pause, "restart after {after}")?;
        },
    }
    match pause_before {
        None => resume_pause.push('.'),
        Some(before) => write!(resume_pause, " and pause before {before}.")?,
    }
    info!("{}", resume_pause);

    let logical_partition_size =
        partitioning.get_logical_partition_size::<M, G, O, R, A>(&algorithm_args);
    if logical_partition_size.get() <= 1 {
        info!("The scenario will be simulated as one monolithic partition.");
    } else {
        info!(
            "The scenario will be simulated across {} logical partitions.",
            logical_partition_size
        );
    }

    let physical_partition_size = partitioning.get_size();
    if physical_partition_size.get() <= 1 {
        info!("The simulation will be run on one processing unit.");
    } else {
        info!(
            "The simulation will be distributed across {} processing units.",
            physical_partition_size
        );
    }

    let will_report_live = partitioning.will_report_live(&event_log);

    if R::ReportSpeciation::VALUE {
        if will_report_live {
            info!("The simulation will report speciation events live.");
        } else {
            info!("The simulation will record speciation events.");
        }
    }

    if R::ReportDispersal::VALUE {
        if will_report_live {
            info!("The simulation will report dispersal events live.");
        } else {
            info!("The simulation will record dispersal events.");
            warn!("Recording dispersal events can be very space-consuming.");
        }
    }

    if R::ReportProgress::VALUE {
        info!("The simulation will report progress events live.");
    }

    if !R::ReportSpeciation::VALUE && !R::ReportDispersal::VALUE && !R::ReportProgress::VALUE {
        warn!("The simulation will report no events.");
    }

    let (result, reporter) = partitioning::dispatch::<M, G, A, O, R, P>(
        partitioning,
        event_log,
        reporter_context,
        sample,
        rng,
        scenario,
        algorithm_args,
        pause_before,
    )?;

    if log::log_enabled!(log::Level::Info) {
        println!("\n");
        println!(
            "{:=^80}",
            if matches!(result, SimulationOutcome::Done { .. }) {
                " Reporter Summary "
            } else {
                " Partial Reporter Summary "
            }
        );
        println!();
    }

    reporter.finalise();

    if log::log_enabled!(log::Level::Info) {
        println!();
        println!(
            "{:=^80}",
            if matches!(result, SimulationOutcome::Done { .. }) {
                " Reporter Summary "
            } else {
                " Partial Reporter Summary "
            }
        );
        println!();
    }

    Ok(result)
}
