use derive_builder::Builder;
use log::LevelFilter;
use serde::Serialize;

use necsim_core::lineage::Lineage;
use necsim_core_bond::NonNegativeF64;

use crate::args::{cli::CommandArgs, utils::ser::BufferingSerializeResult};

mod dispatch;
mod parse;
mod pause;

use dispatch::dispatch;

#[allow(dead_code)] // FIXME: use expect
enum SimulationOutcome {
    Done {
        time: NonNegativeF64,
        steps: u64,
    },
    Paused {
        time: NonNegativeF64,
        steps: u64,
        lineages: Vec<Lineage>,
    },
}

#[expect(clippy::module_name_repetitions)]
pub fn simulate_with_logger(simulate_args: CommandArgs) -> anyhow::Result<()> {
    log::set_max_level(LevelFilter::Info);

    let ron_args = simulate_args.into_config_string();
    parse::fields::parse_and_normalise(&ron_args)?;
    let mut normalised_args = BufferingSimulateArgs::builder();

    let partitioning = parse::partitioning::parse_and_normalise(&ron_args, &mut normalised_args)?;

    #[cfg(feature = "mpi-partitioning")]
    if let crate::args::config::partitioning::Partitioning::Mpi(partitioning) = &partitioning {
        // Only log to stdout/stderr in the MPI root partition
        if !partitioning.peek_is_root() {
            log::set_max_level(log::LevelFilter::Off);
        }
    }

    let pause = parse::pause::parse_and_normalise(&ron_args, &mut normalised_args, &partitioning)?;
    let sample = parse::sample::parse_and_normalise(&ron_args, &mut normalised_args, &pause)?;

    let speciation_probability_per_generation =
        parse::speciation::parse_and_normalise(&ron_args, &mut normalised_args)?;

    let scenario = parse::scenario::parse_and_normalise(&ron_args, &mut normalised_args)?;
    let algorithm =
        parse::algorithm::parse_and_normalise(&ron_args, &mut normalised_args, &partitioning)?;

    let event_log = parse::event_log::parse_and_normalise(
        &ron_args,
        &mut normalised_args,
        &partitioning,
        &sample,
        &pause,
    )?;

    let reporters = parse::reporters::parse_and_normalise(&ron_args, &mut normalised_args)?;

    // Ensure the dynamic reporter plugin libraries are not deallocated prematurely
    reporters.with_lifetime(|reporters| {
        let result = dispatch(
            partitioning,
            event_log,
            reporters,
            speciation_probability_per_generation,
            sample,
            scenario,
            algorithm,
            pause.as_ref().map(|pause| pause.before),
            &ron_args,
            &mut normalised_args,
        )?;

        match &result {
            SimulationOutcome::Done { time, steps } => info!(
                "The simulation finished at time {} after {} steps.\n",
                time.get(),
                steps
            ),
            SimulationOutcome::Paused { time, steps, .. } => info!(
                "The simulation paused at time {} after {} steps.\n",
                time.get(),
                steps
            ),
        }

        if let (Some(pause), SimulationOutcome::Paused { lineages, .. }) = (pause, result) {
            pause::write_resume_config(normalised_args, pause, lineages)?;
        }

        Ok(())
    })
}

#[derive(Serialize, Builder)]
#[builder(setter(into))]
#[serde(rename = "Simulate")]
struct BufferingSimulateArgs {
    speciation: BufferingSerializeResult,
    sample: BufferingSerializeResult,
    pause: BufferingSerializeResult,
    rng: BufferingSerializeResult,
    scenario: BufferingSerializeResult,
    algorithm: BufferingSerializeResult,
    partitioning: BufferingSerializeResult,
    log: BufferingSerializeResult,
    reporters: BufferingSerializeResult,
}

impl BufferingSimulateArgs {
    #[must_use]
    pub fn builder() -> BufferingSimulateArgsBuilder {
        BufferingSimulateArgsBuilder::default()
    }
}
