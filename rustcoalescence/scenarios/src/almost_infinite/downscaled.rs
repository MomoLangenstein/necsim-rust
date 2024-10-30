use std::{marker::PhantomData, num::NonZeroU32};

use serde::{Deserialize, Serialize};

use necsim_core::cogs::{LineageStore, MathsCore, RngCore, SpeciationProbability, TurnoverRate};
use necsim_core_bond::OpenClosedUnitF64 as PositiveUnitF64;
use necsim_partitioning_core::partition::Partition;

use necsim_impls_no_std::{
    cogs::{
        dispersal_sampler::almost_infinite::downscaled::{
            AlmostInfiniteDownscaledDispersalSampler, NonSelfDispersal,
        },
        habitat::almost_infinite::{
            downscaled::{AlmostInfiniteDownscaledHabitat, Log2U16},
            AlmostInfiniteHabitat,
        },
        origin_sampler::{
            pre_sampler::OriginPreSampler,
            singleton_demes::downscaled::AlmostInfiniteDownscaledOriginSampler,
        },
    },
    decomposition::Decomposition,
};

use crate::{Scenario, ScenarioCogs, ScenarioParameters};

pub struct AlmostInfiniteDownscaledScenario<
    M: MathsCore,
    G: RngCore<M>,
    O: Scenario<M, G, Habitat = AlmostInfiniteHabitat<M>>,
> {
    _marker: PhantomData<(M, G, O)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Downscale {
    pub x: Log2U16,
    pub y: Log2U16,
    #[serde(default = "default_downscale_samples")]
    pub samples: NonZeroU32,
    #[serde(default = "default_non_self_dispersal")]
    pub non_self_dispersal: NonSelfDispersal,
}

fn default_downscale_samples() -> NonZeroU32 {
    (NonZeroU32::MIN.saturating_add(1)).saturating_pow(22)
}

fn default_non_self_dispersal() -> NonSelfDispersal {
    NonSelfDispersal::AccurateRejectionSampling
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "AlmostInfiniteDownscaled")]
#[serde(bound = "O::Arguments: serde::Serialize + serde::de::DeserializeOwned")]
pub struct AlmostInfiniteDownscaledArguments<O: ScenarioParameters> {
    #[serde(flatten)]
    pub args: O::Arguments,
    pub downscale: Downscale,
}

impl<
        M: MathsCore,
        G: RngCore<M>,
        O: Scenario<
            M,
            G,
            Habitat = AlmostInfiniteHabitat<M>,
            Decomposition: Decomposition<M, AlmostInfiniteDownscaledHabitat<M>>,
            SpeciationProbability: SpeciationProbability<M, AlmostInfiniteDownscaledHabitat<M>>,
            TurnoverRate: TurnoverRate<M, AlmostInfiniteDownscaledHabitat<M>>,
        >,
    > ScenarioParameters for AlmostInfiniteDownscaledScenario<M, G, O>
{
    type Arguments = AlmostInfiniteDownscaledArguments<O>;
    type Error = O::Error;
}

impl<
        M: MathsCore,
        G: RngCore<M>,
        O: Scenario<
            M,
            G,
            Habitat = AlmostInfiniteHabitat<M>,
            Decomposition: Decomposition<M, AlmostInfiniteDownscaledHabitat<M>>,
            SpeciationProbability: SpeciationProbability<M, AlmostInfiniteDownscaledHabitat<M>>,
            TurnoverRate: TurnoverRate<M, AlmostInfiniteDownscaledHabitat<M>>,
        >,
    > Scenario<M, G> for AlmostInfiniteDownscaledScenario<M, G, O>
{
    type Decomposition = O::Decomposition;
    type DecompositionAuxiliary = O::DecompositionAuxiliary;
    type DispersalSampler = AlmostInfiniteDownscaledDispersalSampler<M, G, O::DispersalSampler>;
    type Habitat = AlmostInfiniteDownscaledHabitat<M>;
    type LineageStore<L: LineageStore<M, Self::Habitat>> = L;
    type OriginSampler<'h, I: Iterator<Item = u64>> = AlmostInfiniteDownscaledOriginSampler<'h, M, O::OriginSampler<'h, I>> where G: 'h, O: 'h;
    type OriginSamplerAuxiliary = O::OriginSamplerAuxiliary;
    type SpeciationProbability = O::SpeciationProbability;
    type TurnoverRate = O::TurnoverRate;

    fn new(
        args: Self::Arguments,
        speciation_probability_per_generation: PositiveUnitF64,
    ) -> Result<ScenarioCogs<M, G, Self>, Self::Error> {
        let ScenarioCogs {
            habitat,
            dispersal_sampler,
            turnover_rate,
            speciation_probability,
            origin_sampler_auxiliary,
            decomposition_auxiliary,
            _marker,
        } = O::new(args.args, speciation_probability_per_generation)?;

        let habitat = AlmostInfiniteDownscaledHabitat::new_with_habitat(
            habitat,
            args.downscale.x,
            args.downscale.y,
        );
        let dispersal_sampler = AlmostInfiniteDownscaledDispersalSampler::new(
            &habitat,
            dispersal_sampler,
            args.downscale.samples,
            args.downscale.non_self_dispersal,
        );

        Ok(ScenarioCogs {
            habitat,
            dispersal_sampler,
            turnover_rate,
            speciation_probability,
            origin_sampler_auxiliary,
            decomposition_auxiliary,
            _marker: PhantomData::<(M, G, Self)>,
        })
    }

    fn sample_habitat<'h, I: Iterator<Item = u64>>(
        habitat: &'h Self::Habitat,
        pre_sampler: OriginPreSampler<M, I>,
        auxiliary: Self::OriginSamplerAuxiliary,
    ) -> Self::OriginSampler<'h, I>
    where
        G: 'h,
    {
        AlmostInfiniteDownscaledOriginSampler::new(
            O::sample_habitat(habitat.unscaled(), pre_sampler, auxiliary),
            habitat,
        )
    }

    fn decompose(
        habitat: &Self::Habitat,
        subdomain: Partition,
        auxiliary: Self::DecompositionAuxiliary,
    ) -> Self::Decomposition {
        O::decompose(habitat.unscaled(), subdomain, auxiliary)
    }
}
