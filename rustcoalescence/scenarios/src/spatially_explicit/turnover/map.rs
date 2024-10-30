#![allow(non_local_definitions)] // FIXME: displaydoc

use std::{convert::TryFrom, marker::PhantomData, path::PathBuf};

use serde::{Deserialize, Serialize, Serializer};

use necsim_core::cogs::{Habitat, LineageStore, MathsCore, RngCore};
use necsim_core_bond::{NonNegativeF64, OpenClosedUnitF64 as PositiveUnitF64};
use necsim_partitioning_core::partition::Partition;

use necsim_impls_no_std::{
    array2d::Array2D,
    cogs::{
        dispersal_sampler::in_memory::{
            packed_separable_alias::InMemoryPackedSeparableAliasDispersalSampler,
            InMemoryDispersalSampler,
        },
        habitat::in_memory::InMemoryHabitat,
        origin_sampler::{in_memory::InMemoryOriginSampler, pre_sampler::OriginPreSampler},
        speciation_probability::uniform::UniformSpeciationProbability,
        turnover_rate::in_memory::{InMemoryTurnoverRate, InMemoryTurnoverRateError},
    },
    decomposition::equal::EqualDecomposition,
};

use necsim_impls_std::cogs::dispersal_sampler::in_memory::InMemoryDispersalSamplerError;

use crate::{Scenario, ScenarioCogs, ScenarioParameters};

use super::super::maps::{self, MapLoadingMode};

#[derive(thiserror::Error, displaydoc::Display, Debug)]
pub enum SpatiallyExplicitTurnoverMapScenarioError {
    /// invalid habitat map: no habitable locations
    EmptyHabitatMap,
    /// invalid dispersal map: {0}
    DispersalMap(InMemoryDispersalSamplerError),
    /// invalid turnover map: {0}
    TurnoverMap(InMemoryTurnoverRateError),
}

#[expect(clippy::empty_enum)]
pub enum SpatiallyExplicitTurnoverMapScenario {}

impl ScenarioParameters for SpatiallyExplicitTurnoverMapScenario {
    type Arguments = SpatiallyExplicitTurnoverMapArguments;
    type Error = SpatiallyExplicitTurnoverMapScenarioError;
}

impl<M: MathsCore, G: RngCore<M>> Scenario<M, G> for SpatiallyExplicitTurnoverMapScenario {
    type Decomposition = EqualDecomposition<M, Self::Habitat>;
    type DecompositionAuxiliary = ();
    type DispersalSampler = InMemoryPackedSeparableAliasDispersalSampler<M, Self::Habitat, G>;
    type Habitat = InMemoryHabitat<M>;
    type LineageStore<L: LineageStore<M, Self::Habitat>> = L;
    type OriginSampler<'h, I: Iterator<Item = u64>> = InMemoryOriginSampler<'h, M, I> where G: 'h;
    type OriginSamplerAuxiliary = ();
    type SpeciationProbability = UniformSpeciationProbability;
    type TurnoverRate = InMemoryTurnoverRate;

    fn new(
        args: Self::Arguments,
        speciation_probability_per_generation: PositiveUnitF64,
    ) -> Result<ScenarioCogs<M, G, Self>, Self::Error> {
        let habitat = InMemoryHabitat::try_new(args.habitat_map)
            .ok_or(SpatiallyExplicitTurnoverMapScenarioError::EmptyHabitatMap)?;
        let turnover_rate = InMemoryTurnoverRate::new(args.turnover_map, &habitat)
            .map_err(SpatiallyExplicitTurnoverMapScenarioError::TurnoverMap)?;
        let speciation_probability =
            UniformSpeciationProbability::new(speciation_probability_per_generation.into());
        let dispersal_sampler =
            InMemoryPackedSeparableAliasDispersalSampler::new(&args.dispersal_map, &habitat)
                .map_err(|err| {
                    SpatiallyExplicitTurnoverMapScenarioError::DispersalMap(err.into())
                })?;

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
        InMemoryOriginSampler::new(pre_sampler, habitat)
    }

    fn decompose(
        habitat: &Self::Habitat,
        subdomain: Partition,
        _auxiliary: Self::DecompositionAuxiliary,
    ) -> Self::Decomposition {
        match EqualDecomposition::weight(habitat, subdomain) {
            Ok(decomposition) => decomposition,
            Err(decomposition) => {
                warn!(
                    "Spatially explicit habitat of size {}x{} could not be partitioned into {} \
                     partition(s).",
                    habitat.get_extent().width(),
                    habitat.get_extent().height(),
                    subdomain.size().get(),
                );

                decomposition
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "SpatiallyExplicitTurnoverMapArgumentsRaw")]
pub struct SpatiallyExplicitTurnoverMapArguments {
    pub habitat_path: PathBuf,
    pub habitat_map: Array2D<u32>,
    pub dispersal_path: PathBuf,
    pub dispersal_map: Array2D<NonNegativeF64>,
    pub turnover_path: PathBuf,
    pub turnover_map: Array2D<NonNegativeF64>,
    pub loading_mode: MapLoadingMode,
}

impl SpatiallyExplicitTurnoverMapArguments {
    #[expect(clippy::missing_errors_doc)]
    pub fn try_load(
        habitat_path: PathBuf,
        dispersal_path: PathBuf,
        turnover_path: PathBuf,
        loading_mode: MapLoadingMode,
    ) -> Result<Self, String> {
        info!(
            "Starting to load the dispersal map {:?} ...",
            dispersal_path
        );

        let mut dispersal_map = maps::load_dispersal_map(&dispersal_path, loading_mode)
            .map_err(|err| format!("{err:?}"))?;

        info!(
            "Successfully loaded the dispersal map {:?} with dimensions {}x{} [cols x rows].",
            &dispersal_path,
            dispersal_map.num_columns(),
            dispersal_map.num_rows()
        );

        info!("Starting to load the turnover map {:?} ...", &turnover_path);

        let turnover_map = maps::load_turnover_map(&turnover_path, loading_mode)
            .map_err(|err| format!("{err:?}"))?;

        info!(
            "Successfully loaded the turnover map {:?} with dimensions {}x{} [cols x rows].",
            &turnover_path,
            turnover_map.num_columns(),
            turnover_map.num_rows()
        );

        info!("Starting to load the habitat map {:?} ...", &habitat_path);

        let habitat_map = maps::load_habitat_map(
            &habitat_path,
            Some(&turnover_map),
            &mut dispersal_map,
            loading_mode,
        )
        .map_err(|err| format!("{err:?}"))?;

        info!(
            "Successfully loaded the habitat map {:?} with dimensions {}x{} [cols x rows].",
            &habitat_path,
            habitat_map.num_columns(),
            habitat_map.num_rows()
        );

        Ok(SpatiallyExplicitTurnoverMapArguments {
            habitat_path,
            habitat_map,
            dispersal_path,
            dispersal_map,
            turnover_path,
            turnover_map,
            loading_mode,
        })
    }
}

impl Serialize for SpatiallyExplicitTurnoverMapArguments {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SpatiallyExplicitTurnoverMapArgumentsRaw {
            habitat_map: self.habitat_path.clone(),
            dispersal_map: self.dispersal_path.clone(),
            turnover_map: self.turnover_path.clone(),
            loading_mode: self.loading_mode,
        }
        .serialize(serializer)
    }
}

impl TryFrom<SpatiallyExplicitTurnoverMapArgumentsRaw> for SpatiallyExplicitTurnoverMapArguments {
    type Error = String;

    fn try_from(raw: SpatiallyExplicitTurnoverMapArgumentsRaw) -> Result<Self, Self::Error> {
        Self::try_load(
            raw.habitat_map,
            raw.dispersal_map,
            raw.turnover_map,
            raw.loading_mode,
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "SpatiallyExplicitTurnoverMap")]
struct SpatiallyExplicitTurnoverMapArgumentsRaw {
    #[serde(rename = "habitat", alias = "habitat_map")]
    habitat_map: PathBuf,

    #[serde(rename = "dispersal", alias = "dispersal_map")]
    dispersal_map: PathBuf,

    #[serde(rename = "turnover", alias = "turnover_map")]
    turnover_map: PathBuf,

    #[serde(default)]
    #[serde(rename = "mode", alias = "loading_mode")]
    loading_mode: MapLoadingMode,
}
