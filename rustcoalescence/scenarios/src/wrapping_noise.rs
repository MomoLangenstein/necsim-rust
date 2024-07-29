use std::{marker::PhantomData, num::NonZeroUsize};

use serde::{Deserialize, Serialize};

use necsim_core::{
    cogs::{LineageStore, MathsCore, RngCore},
    landscape::LandscapeExtent,
};
use necsim_core_bond::{ClosedUnitF64, NonNegativeF64, OpenClosedUnitF64 as PositiveUnitF64};
use necsim_partitioning_core::partition::Partition;

use necsim_impls_no_std::{
    cogs::{
        dispersal_sampler::wrapping_noise::WrappingNoiseApproximateNormalDispersalSampler,
        habitat::wrapping_noise::WrappingNoiseHabitat,
        lineage_store::coherent::globally::singleton_demes::SingletonDemesLineageStore,
        origin_sampler::{
            pre_sampler::OriginPreSampler,
            singleton_demes::rectangle::SingletonDemesRectangleOriginSampler,
        },
        speciation_probability::uniform::UniformSpeciationProbability,
        turnover_rate::uniform::UniformTurnoverRate,
    },
    decomposition::radial::RadialDecomposition,
};

use crate::{Scenario, ScenarioCogs, ScenarioParameters};

#[expect(clippy::module_name_repetitions, clippy::empty_enum)]
#[derive(Clone)]
pub enum WrappingNoiseScenario {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[expect(clippy::module_name_repetitions)]
#[serde(deny_unknown_fields)]
#[serde(rename = "WrappingNoise")]
pub struct WrappingNoiseArguments {
    pub seed: i64,
    pub coverage: ClosedUnitF64,
    pub scale: PositiveUnitF64,
    pub persistence: PositiveUnitF64,
    pub octaves: NonZeroUsize,
    pub sample: Sample,
    pub sigma: NonNegativeF64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Sample {
    #[serde(alias = "Extent")]
    Rectangle(LandscapeExtent),
}

impl ScenarioParameters for WrappingNoiseScenario {
    type Arguments = WrappingNoiseArguments;
    type Error = !;
}

impl<M: MathsCore, G: RngCore<M>> Scenario<M, G> for WrappingNoiseScenario {
    type Decomposition = RadialDecomposition;
    type DecompositionAuxiliary = ();
    type DispersalSampler = WrappingNoiseApproximateNormalDispersalSampler<M, G>;
    type Habitat = WrappingNoiseHabitat<M>;
    type LineageStore<L: LineageStore<M, Self::Habitat>> =
        SingletonDemesLineageStore<M, Self::Habitat>;
    type OriginSampler<'h, I: Iterator<Item = u64>> = SingletonDemesRectangleOriginSampler<'h, M, Self::Habitat, I> where G: 'h;
    type OriginSamplerAuxiliary = (LandscapeExtent,);
    type SpeciationProbability = UniformSpeciationProbability;
    type TurnoverRate = UniformTurnoverRate;

    fn new(
        args: Self::Arguments,
        speciation_probability_per_generation: PositiveUnitF64,
    ) -> Result<ScenarioCogs<M, G, Self>, Self::Error> {
        let habitat = WrappingNoiseHabitat::new(
            args.seed,
            args.coverage,
            args.scale,
            args.persistence,
            args.octaves,
        );
        let dispersal_sampler = WrappingNoiseApproximateNormalDispersalSampler::new(args.sigma);
        let turnover_rate = UniformTurnoverRate::default();
        let speciation_probability =
            UniformSpeciationProbability::new(speciation_probability_per_generation.into());

        let Sample::Rectangle(sample) = args.sample;

        Ok(ScenarioCogs {
            habitat,
            dispersal_sampler,
            turnover_rate,
            speciation_probability,
            origin_sampler_auxiliary: (sample,),
            decomposition_auxiliary: (),
            _marker: PhantomData::<(M, G, Self)>,
        })
    }

    fn sample_habitat<'h, I: Iterator<Item = u64>>(
        habitat: &'h Self::Habitat,
        pre_sampler: OriginPreSampler<M, I>,
        (sample,): Self::OriginSamplerAuxiliary,
    ) -> Self::OriginSampler<'_, I>
    where
        G: 'h,
    {
        SingletonDemesRectangleOriginSampler::new(pre_sampler, habitat, sample)
    }

    fn decompose(
        _habitat: &Self::Habitat,
        subdomain: Partition,
        _auxiliary: Self::DecompositionAuxiliary,
    ) -> Self::Decomposition {
        RadialDecomposition::new(subdomain)
    }
}
