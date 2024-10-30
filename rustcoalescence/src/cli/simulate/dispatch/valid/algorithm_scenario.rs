use std::marker::PhantomData;

use necsim_core::{
    cogs::{MathsCore, RngCore},
    reporter::Reporter,
};
use necsim_core_bond::{NonNegativeF64, OpenClosedUnitF64 as PositiveUnitF64};
use necsim_impls_std::event_log::recorder::EventLogConfig;
use necsim_partitioning_core::reporter::ReporterContext;

use rustcoalescence_algorithms::AlgorithmDefaults;

#[cfg(feature = "cuda-algorithm")]
use rustcoalescence_algorithms_cuda::CudaAlgorithm;
#[cfg(feature = "gillespie-algorithms")]
use rustcoalescence_algorithms_gillespie::{
    event_skipping::EventSkippingAlgorithm, gillespie::GillespieAlgorithm,
};
#[cfg(feature = "independent-algorithm")]
use rustcoalescence_algorithms_independent::IndependentAlgorithm;

#[cfg(feature = "almost-infinite-clark2dt-dispersal-scenario")]
use rustcoalescence_scenarios::almost_infinite::clark2dt::AlmostInfiniteClark2DtDispersalScenario;
#[cfg(all(
    any(
        feature = "almost-infinite-clark2dt-dispersal-scenario",
        feature = "almost-infinite-normal-dispersal-scenario",
    ),
    feature = "almost-infinite-downscaled-scenario",
))]
use rustcoalescence_scenarios::almost_infinite::downscaled::AlmostInfiniteDownscaledScenario;
#[cfg(feature = "almost-infinite-normal-dispersal-scenario")]
use rustcoalescence_scenarios::almost_infinite::normal::AlmostInfiniteNormalDispersalScenario;
#[cfg(feature = "non-spatial-scenario")]
use rustcoalescence_scenarios::non_spatial::NonSpatialScenario;
#[cfg(feature = "spatially-explicit-turnover-map-scenario")]
use rustcoalescence_scenarios::spatially_explicit::map::SpatiallyExplicitTurnoverMapScenario;
#[cfg(feature = "spatially-explicit-uniform-turnover-scenario")]
use rustcoalescence_scenarios::spatially_explicit::uniform::SpatiallyExplicitUniformTurnoverScenario;
#[cfg(feature = "spatially-implicit-scenario")]
use rustcoalescence_scenarios::spatially_implicit::SpatiallyImplicitScenario;
#[cfg(feature = "wrapping-noise-scenario")]
use rustcoalescence_scenarios::wrapping_noise::WrappingNoiseScenario;
use rustcoalescence_scenarios::Scenario;

use crate::{
    args::config::{
        algorithm::Algorithm as AlgorithmArgs, partitioning::Partitioning,
        sample::Sample as SampleArgs, scenario::Scenario as ScenarioArgs,
    },
    cli::simulate::SimulationOutcome,
};

use super::{super::super::BufferingSimulateArgsBuilder, rng};

macro_rules! match_scenario_algorithm {
    (
        ($algorithm:expr, $scenario:expr => $algscen:ident : $algscenty:ident) {
            $($(#[$algmeta:meta])* $algpat:pat => $algcode:block),*
            [<=>]
            $($(#[$scenmeta:meta])* $scenpat:pat => $scencode:block => $scenty:ty),*
        }
    ) => {
        match_scenario_algorithm! {
            impl ($algorithm, $scenario => $algscen : $algscenty) {
                $($(#[$algmeta])* $algpat => $algcode),*
                [<=>]
                $($(#[$scenmeta])* $scenpat => $scencode => $scenty),*
                [<=>]
            }
        }
    };
    (
        impl ($algorithm:expr, $scenario:expr => $algscen:ident : $algscenty:ident) {
            $(#[$algmeta:meta])* $algpat:pat => $algcode:block,
            $($(#[$algmetarem:meta])* $algpatrem:pat => $algcoderem:block),+
            [<=>]
            $($(#[$scenmeta:meta])* $scenpat:pat => $scencode:block => $scenty:ty),*
            [<=>]
            $($tail:tt)*
        }
    ) => {
        match_scenario_algorithm! {
            impl ($algorithm, $scenario => $algscen : $algscenty) {
                $($(#[$algmetarem])* $algpatrem => $algcoderem),+
                [<=>]
                $($(#[$scenmeta])* $scenpat => $scencode => $scenty),*
                [<=>]
                $($tail)*
                $(#[$algmeta])* $algpat => {
                    match $scenario {
                        $($(#[$scenmeta])* $scenpat => {
                            type $algscenty<M, G> = PhantomData<(M, G, $scenty)>;
                            let $algscen = $scencode;
                            $algcode
                        }),*
                    }
                }
            }
        }
    };
    (
        impl ($algorithm:expr, $scenario:expr => $algscen:ident : $algscenty:ident) {
            $(#[$algmeta:meta])* $algpat:pat => $algcode:block
            [<=>]
            $($(#[$scenmeta:meta])* $scenpat:pat => $scencode:block => $scenty:ty),*
            [<=>]
            $($tail:tt)*
        }
    ) => {
        match $algorithm {
            $($tail)*
            $(#[$algmeta])* $algpat => {
                match $scenario {
                    $($(#[$scenmeta])* $scenpat => {
                        type $algscenty<M, G> = PhantomData<(M, G, $scenty)>;
                        let $algscen = $scencode;
                        $algcode
                    }),*
                }
            }
        }
    };
}

#[expect(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn dispatch<R: Reporter, P: ReporterContext<Reporter = R>>(
    partitioning: Partitioning,
    event_log: Option<EventLogConfig>,
    reporter_context: P,

    speciation_probability_per_generation: PositiveUnitF64,
    sample: SampleArgs,
    scenario: ScenarioArgs,
    algorithm: AlgorithmArgs,
    pause_before: Option<NonNegativeF64>,

    ron_args: &str,
    normalised_args: &mut BufferingSimulateArgsBuilder,
) -> anyhow::Result<SimulationOutcome> {
    match_scenario_algorithm!(
        (algorithm, scenario => scenario: ScenarioTy)
    {
        #[cfg(feature = "gillespie-algorithms")]
        AlgorithmArgs::Gillespie(algorithm_args) => {
            rng::dispatch::<
                <GillespieAlgorithm as AlgorithmDefaults>::MathsCore,
                <GillespieAlgorithm as AlgorithmDefaults>::Rng<_>,
                GillespieAlgorithm, <ScenarioTy<
                    <GillespieAlgorithm as AlgorithmDefaults>::MathsCore,
                    <GillespieAlgorithm as AlgorithmDefaults>::Rng<_>,
                > as ScenarioDispatch>::Scenario, R, P,
            >(
                partitioning, event_log, reporter_context,
                sample, algorithm_args, scenario,
                pause_before, ron_args, normalised_args,
            )
        },
        #[cfg(feature = "gillespie-algorithms")]
        AlgorithmArgs::EventSkipping(algorithm_args) => {
            rng::dispatch::<
                <EventSkippingAlgorithm as AlgorithmDefaults>::MathsCore,
                <EventSkippingAlgorithm as AlgorithmDefaults>::Rng<_>,
                EventSkippingAlgorithm, <ScenarioTy<
                    <EventSkippingAlgorithm as AlgorithmDefaults>::MathsCore,
                    <EventSkippingAlgorithm as AlgorithmDefaults>::Rng<_>,
                > as ScenarioDispatch>::Scenario, R, P,
            >(
                partitioning, event_log, reporter_context,
                sample, algorithm_args, scenario,
                pause_before, ron_args, normalised_args,
            )
        },
        #[cfg(feature = "independent-algorithm")]
        AlgorithmArgs::Independent(algorithm_args) => {
            rng::dispatch::<
                <IndependentAlgorithm as AlgorithmDefaults>::MathsCore,
                <IndependentAlgorithm as AlgorithmDefaults>::Rng<_>,
                IndependentAlgorithm, <ScenarioTy<
                    <IndependentAlgorithm as AlgorithmDefaults>::MathsCore,
                    <IndependentAlgorithm as AlgorithmDefaults>::Rng<_>,
                > as ScenarioDispatch>::Scenario, R, P,
            >(
                partitioning, event_log, reporter_context,
                sample, algorithm_args, scenario,
                pause_before, ron_args, normalised_args,
            )
        },
        #[cfg(feature = "cuda-algorithm")]
        AlgorithmArgs::Cuda(algorithm_args) => {
            rng::dispatch::<
                <CudaAlgorithm as AlgorithmDefaults>::MathsCore,
                <CudaAlgorithm as AlgorithmDefaults>::Rng<_>,
                CudaAlgorithm, <ScenarioTy<
                    <CudaAlgorithm as AlgorithmDefaults>::MathsCore,
                    <CudaAlgorithm as AlgorithmDefaults>::Rng<_>,
                > as ScenarioDispatch>::Scenario, R, P,
            >(
                partitioning, event_log, reporter_context,
                sample, algorithm_args, scenario,
                pause_before, ron_args, normalised_args,
            )
        }
        [<=>]
        #[cfg(feature = "spatially-explicit-uniform-turnover-scenario")]
        ScenarioArgs::SpatiallyExplicitUniformTurnover(scenario_args) => {
            SpatiallyExplicitUniformTurnoverScenario::new(
                scenario_args,
                speciation_probability_per_generation,
            )?
        } => SpatiallyExplicitUniformTurnoverScenario,
        #[cfg(feature = "spatially-explicit-turnover-map-scenario")]
        ScenarioArgs::SpatiallyExplicitTurnoverMap(scenario_args) => {
            SpatiallyExplicitTurnoverMapScenario::new(
                scenario_args,
                speciation_probability_per_generation,
            )?
        } => SpatiallyExplicitTurnoverMapScenario,
        #[cfg(feature = "non-spatial-scenario")]
        ScenarioArgs::NonSpatial(scenario_args) => {
            NonSpatialScenario::new(
                scenario_args,
                speciation_probability_per_generation,
            )
            .into_ok()
        } => NonSpatialScenario,
        #[cfg(feature = "almost-infinite-normal-dispersal-scenario")]
        ScenarioArgs::AlmostInfiniteNormalDispersal(scenario_args) => {
            AlmostInfiniteNormalDispersalScenario::new(
                scenario_args,
                speciation_probability_per_generation,
            )
            .into_ok()
        } => AlmostInfiniteNormalDispersalScenario,
        #[cfg(feature = "almost-infinite-clark2dt-dispersal-scenario")]
        ScenarioArgs::AlmostInfiniteClark2DtDispersal(scenario_args) => {
            AlmostInfiniteClark2DtDispersalScenario::new(
                scenario_args,
                speciation_probability_per_generation,
            )
            .into_ok()
        } => AlmostInfiniteClark2DtDispersalScenario,
        #[cfg(all(
            feature = "almost-infinite-normal-dispersal-scenario",
            feature = "almost-infinite-downscaled-scenario",
        ))]
        ScenarioArgs::AlmostInfiniteDownscaledNormalDispersal(scenario_args) => {
            AlmostInfiniteDownscaledScenario::new(
                scenario_args,
                speciation_probability_per_generation,
            )
            .into_ok()
        } => AlmostInfiniteDownscaledScenario<M, G, AlmostInfiniteNormalDispersalScenario>,
        #[cfg(all(
            feature = "almost-infinite-clark2dt-dispersal-scenario",
            feature = "almost-infinite-downscaled-scenario",
        ))]
        ScenarioArgs::AlmostInfiniteDownscaledClark2DtDispersal(scenario_args) => {
            AlmostInfiniteDownscaledScenario::new(
                scenario_args,
                speciation_probability_per_generation,
            )
            .into_ok()
        } => AlmostInfiniteDownscaledScenario<M, G, AlmostInfiniteClark2DtDispersalScenario>,
        #[cfg(feature = "spatially-implicit-scenario")]
        ScenarioArgs::SpatiallyImplicit(scenario_args) => {
            SpatiallyImplicitScenario::new(
                scenario_args,
                speciation_probability_per_generation,
            )
            .into_ok()
        } => SpatiallyImplicitScenario,
        #[cfg(feature = "wrapping-noise-scenario")]
        ScenarioArgs::WrappingNoise(scenario_args) => {
            WrappingNoiseScenario::new(
                scenario_args,
                speciation_probability_per_generation,
            )
            .into_ok()
        } => WrappingNoiseScenario
    })
}

trait ScenarioDispatch {
    type Scenario;
}

impl<M: MathsCore, G: RngCore<M>, O: Scenario<M, G>> ScenarioDispatch for PhantomData<(M, G, O)> {
    type Scenario = O;
}
