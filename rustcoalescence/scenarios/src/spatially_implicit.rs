use std::{marker::PhantomData, num::NonZeroU32};

use serde::{Deserialize, Serialize};

use necsim_core::cogs::{LineageStore, MathsCore, RngCore};
use necsim_core_bond::{OffByOneU32, OpenClosedUnitF64 as PositiveUnitF64};
use necsim_partitioning_core::partition::Partition;

use necsim_impls_no_std::{
    cogs::{
        dispersal_sampler::spatially_implicit::SpatiallyImplicitDispersalSampler,
        habitat::spatially_implicit::SpatiallyImplicitHabitat,
        origin_sampler::{
            pre_sampler::OriginPreSampler, spatially_implicit::SpatiallyImplicitOriginSampler,
        },
        speciation_probability::spatially_implicit::SpatiallyImplicitSpeciationProbability,
        turnover_rate::uniform::UniformTurnoverRate,
    },
    decomposition::modulo::ModuloDecomposition,
};

use crate::{Scenario, ScenarioCogs, ScenarioParameters};

#[expect(clippy::module_name_repetitions, clippy::empty_enum)]
pub enum SpatiallyImplicitScenario {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[expect(clippy::module_name_repetitions)]
#[serde(deny_unknown_fields)]
#[serde(rename = "SpatiallyImplicit")]
pub struct SpatiallyImplicitArguments {
    pub local_area: (OffByOneU32, OffByOneU32),
    pub local_deme: NonZeroU32,
    pub meta_area: (OffByOneU32, OffByOneU32),
    pub meta_deme: NonZeroU32,

    #[serde(alias = "migration")]
    pub migration_probability_per_generation: PositiveUnitF64,
}

impl ScenarioParameters for SpatiallyImplicitScenario {
    type Arguments = SpatiallyImplicitArguments;
    type Error = !;
}

impl<M: MathsCore, G: RngCore<M>> Scenario<M, G> for SpatiallyImplicitScenario {
    type Decomposition = ModuloDecomposition;
    type DecompositionAuxiliary = ();
    type DispersalSampler = SpatiallyImplicitDispersalSampler<M, G>;
    type Habitat = SpatiallyImplicitHabitat<M>;
    type LineageStore<L: LineageStore<M, Self::Habitat>> = L;
    type OriginSampler<'h, I: Iterator<Item = u64>> = SpatiallyImplicitOriginSampler<'h, M, I> where G: 'h;
    type OriginSamplerAuxiliary = ();
    type SpeciationProbability = SpatiallyImplicitSpeciationProbability;
    type TurnoverRate = UniformTurnoverRate;

    fn new(
        args: Self::Arguments,
        speciation_probability_per_generation: PositiveUnitF64,
    ) -> Result<ScenarioCogs<M, G, Self>, Self::Error> {
        let habitat = SpatiallyImplicitHabitat::new(
            args.local_area,
            args.local_deme,
            args.meta_area,
            args.meta_deme,
        );
        let dispersal_sampler =
            SpatiallyImplicitDispersalSampler::new(args.migration_probability_per_generation);
        let turnover_rate = UniformTurnoverRate::default();
        let speciation_probability =
            SpatiallyImplicitSpeciationProbability::new(speciation_probability_per_generation);

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
        SpatiallyImplicitOriginSampler::new(pre_sampler, habitat)
    }

    fn decompose(
        _habitat: &Self::Habitat,
        subdomain: Partition,
        _auxiliary: Self::DecompositionAuxiliary,
    ) -> Self::Decomposition {
        ModuloDecomposition::new(subdomain)
    }
}
