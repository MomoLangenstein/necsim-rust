use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use necsim_core::cogs::{LineageStore, MathsCore, RngCore};
use necsim_core_bond::{NonNegativeF64, OpenClosedUnitF64 as PositiveUnitF64};
use necsim_partitioning_core::partition::Partition;

use necsim_impls_no_std::{
    cogs::{
        dispersal_sampler::almost_infinite::normal::AlmostInfiniteNormalDispersalSampler,
        habitat::almost_infinite::AlmostInfiniteHabitat,
        lineage_store::coherent::globally::singleton_demes::SingletonDemesLineageStore,
        origin_sampler::{
            pre_sampler::OriginPreSampler, singleton_demes::SingletonDemesOriginSampler,
        },
        speciation_probability::uniform::UniformSpeciationProbability,
        turnover_rate::uniform::UniformTurnoverRate,
    },
    decomposition::radial::RadialDecomposition,
};

use crate::{Scenario, ScenarioCogs, ScenarioParameters};

use super::Sample;

#[expect(clippy::empty_enum)]
#[derive(Debug)]
pub enum AlmostInfiniteNormalDispersalScenario {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "AlmostInfiniteNormalDispersal")]
pub struct AlmostInfiniteNormalDispersalArguments {
    pub sample: Sample,
    pub sigma: NonNegativeF64,
}

impl ScenarioParameters for AlmostInfiniteNormalDispersalScenario {
    type Arguments = AlmostInfiniteNormalDispersalArguments;
    type Error = !;
}

impl<M: MathsCore, G: RngCore<M>> Scenario<M, G> for AlmostInfiniteNormalDispersalScenario {
    type Decomposition = RadialDecomposition;
    type DecompositionAuxiliary = ();
    type DispersalSampler = AlmostInfiniteNormalDispersalSampler<M, G>;
    type Habitat = AlmostInfiniteHabitat<M>;
    type LineageStore<L: LineageStore<M, Self::Habitat>> =
        SingletonDemesLineageStore<M, Self::Habitat>;
    type OriginSampler<'h, I: Iterator<Item = u64>> = SingletonDemesOriginSampler<'h, M, Self::Habitat, I> where G: 'h;
    type OriginSamplerAuxiliary = (Sample,);
    type SpeciationProbability = UniformSpeciationProbability;
    type TurnoverRate = UniformTurnoverRate;

    fn new(
        args: Self::Arguments,
        speciation_probability_per_generation: PositiveUnitF64,
    ) -> Result<ScenarioCogs<M, G, Self>, Self::Error> {
        let habitat = AlmostInfiniteHabitat::default();
        let dispersal_sampler = AlmostInfiniteNormalDispersalSampler::new(args.sigma);
        let turnover_rate = UniformTurnoverRate::default();
        let speciation_probability =
            UniformSpeciationProbability::new(speciation_probability_per_generation.into());

        Ok(ScenarioCogs {
            habitat,
            dispersal_sampler,
            turnover_rate,
            speciation_probability,
            origin_sampler_auxiliary: (args.sample,),
            decomposition_auxiliary: (),
            _marker: PhantomData::<(M, G, Self)>,
        })
    }

    fn sample_habitat<'h, I: Iterator<Item = u64>>(
        habitat: &'h Self::Habitat,
        pre_sampler: OriginPreSampler<M, I>,
        (sample,): Self::OriginSamplerAuxiliary,
    ) -> Self::OriginSampler<'h, I>
    where
        G: 'h,
    {
        sample.into_origin_sampler(habitat, pre_sampler)
    }

    fn decompose(
        _habitat: &Self::Habitat,
        subdomain: Partition,
        _auxiliary: Self::DecompositionAuxiliary,
    ) -> Self::Decomposition {
        RadialDecomposition::new(subdomain)
    }
}
