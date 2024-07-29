use necsim_impls_std::event_log::recorder::EventLogConfig;
use necsim_partitioning_core::reporter::ReporterContext;
use tiny_keccak::{Hasher, Keccak};

use rustcoalescence_algorithms::{
    result::SimulationOutcome as AlgorithmOutcome, AlgorithmDispatch,
};

use necsim_core::{
    cogs::{MathsCore, RngCore, SeedableRng},
    reporter::Reporter,
};
use necsim_core_bond::NonNegativeF64;

use rustcoalescence_scenarios::{Scenario, ScenarioCogs};

use crate::{
    args::config::{
        partitioning::Partitioning,
        rng::{Base32RngState, Rng as RngArgs},
        sample::Sample,
    },
    cli::simulate::parse,
};

use super::{
    super::super::{BufferingSimulateArgsBuilder, SimulationOutcome},
    info,
};

#[expect(clippy::too_many_arguments)]
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
    algorithm_args: A::Arguments,
    scenario: ScenarioCogs<M, G, O>,
    pause_before: Option<NonNegativeF64>,

    ron_args: &str,
    normalised_args: &mut BufferingSimulateArgsBuilder,
) -> anyhow::Result<SimulationOutcome>
where
    Result<AlgorithmOutcome<M, G>, A::Error>: anyhow::Context<AlgorithmOutcome<M, G>, A::Error>,
{
    let rng: G = match parse::rng::parse_and_normalise(
        ron_args,
        normalised_args,
        partitioning.get_logical_partition_size::<M, G, O, R, A>(&algorithm_args),
    )? {
        RngArgs::Seed(seed) => SeedableRng::seed_from_u64(seed),
        RngArgs::Sponge(bytes) => {
            let mut seed = G::Seed::default();

            let mut sponge = Keccak::v256();
            sponge.update(&bytes);
            sponge.finalize(seed.as_mut());

            RngCore::from_seed(seed)
        },
        RngArgs::State(state) => state.into(),
    };

    let result = info::dispatch::<M, G, A, O, R, P>(
        partitioning,
        event_log,
        reporter_context,
        sample,
        rng,
        scenario,
        algorithm_args,
        pause_before,
        normalised_args,
    )?;

    match result {
        AlgorithmOutcome::Done { time, steps } => Ok(SimulationOutcome::Done { time, steps }),
        AlgorithmOutcome::Paused {
            time,
            steps,
            lineages,
            rng: paused_rng,
            ..
        } => {
            normalised_args.rng(&RngArgs::State(Base32RngState::from(paused_rng)));

            Ok(SimulationOutcome::Paused {
                time,
                steps,
                lineages,
            })
        },
    }
}
