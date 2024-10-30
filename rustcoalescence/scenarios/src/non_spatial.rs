use std::{marker::PhantomData, num::NonZeroU32};

use serde::{Deserialize, Serialize};

use necsim_core::cogs::{LineageStore, MathsCore, RngCore};
use necsim_core_bond::{OffByOneU32, OpenClosedUnitF64 as PositiveUnitF64};
use necsim_partitioning_core::partition::Partition;

use necsim_impls_no_std::{
    cogs::{
        dispersal_sampler::non_spatial::NonSpatialDispersalSampler,
        habitat::non_spatial::NonSpatialHabitat,
        origin_sampler::{non_spatial::NonSpatialOriginSampler, pre_sampler::OriginPreSampler},
        speciation_probability::uniform::UniformSpeciationProbability,
        turnover_rate::uniform::UniformTurnoverRate,
    },
    decomposition::modulo::ModuloDecomposition,
};

use crate::{Scenario, ScenarioCogs, ScenarioParameters};

#[expect(clippy::module_name_repetitions, clippy::empty_enum)]
pub enum NonSpatialScenario {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[expect(clippy::module_name_repetitions)]
#[serde(deny_unknown_fields)]
#[serde(rename = "NonSpatial")]
pub struct NonSpatialArguments {
    pub area: (OffByOneU32, OffByOneU32),
    pub deme: NonZeroU32,
}

impl ScenarioParameters for NonSpatialScenario {
    type Arguments = NonSpatialArguments;
    type Error = !;
}

impl<M: MathsCore, G: RngCore<M>> Scenario<M, G> for NonSpatialScenario {
    type Decomposition = ModuloDecomposition;
    type DecompositionAuxiliary = ();
    type DispersalSampler = NonSpatialDispersalSampler<M, G>;
    type Habitat = NonSpatialHabitat<M>;
    type LineageStore<L: LineageStore<M, Self::Habitat>> = L;
    type OriginSampler<'h, I: Iterator<Item = u64>> = NonSpatialOriginSampler<'h, M, I> where G: 'h;
    type OriginSamplerAuxiliary = ();
    type SpeciationProbability = UniformSpeciationProbability;
    type TurnoverRate = UniformTurnoverRate;

    fn new(
        args: Self::Arguments,
        speciation_probability_per_generation: PositiveUnitF64,
    ) -> Result<ScenarioCogs<M, G, Self>, Self::Error> {
        let habitat = NonSpatialHabitat::new(args.area, args.deme);
        let dispersal_sampler = NonSpatialDispersalSampler::default();
        let turnover_rate = UniformTurnoverRate::default();
        let speciation_probability =
            UniformSpeciationProbability::new(speciation_probability_per_generation.into());

        Ok(ScenarioCogs {
            habitat,
            dispersal_sampler,
            turnover_rate,
            speciation_probability,
            origin_sampler_auxiliary: (),
            decomposition_auxiliary: (),
            _marker: PhantomData::<(M, G, Self)>,
        })
    }

    fn sample_habitat<'h, I: Iterator<Item = u64>>(
        habitat: &'h Self::Habitat,
        pre_sampler: OriginPreSampler<M, I>,
        _auxiliary: Self::OriginSamplerAuxiliary,
    ) -> Self::OriginSampler<'h, I>
    where
        G: 'h,
    {
        NonSpatialOriginSampler::new(pre_sampler, habitat)
    }

    fn decompose(
        _habitat: &Self::Habitat,
        subdomain: Partition,
        _auxiliary: Self::DecompositionAuxiliary,
    ) -> Self::Decomposition {
        ModuloDecomposition::new(subdomain)
    }
}
