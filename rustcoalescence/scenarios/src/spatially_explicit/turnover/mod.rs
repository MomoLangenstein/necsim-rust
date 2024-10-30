use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::maps::MapLoadingMode;

pub mod map;
pub mod uniform;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "SpatiallyExplicit")]
pub struct SpatiallyExplicitArguments {
    #[serde(rename = "habitat", alias = "habitat_map")]
    habitat_map: PathBuf,

    #[serde(rename = "dispersal", alias = "dispersal_map")]
    dispersal_map: PathBuf,

    #[cfg_attr(feature = "spatially-explicit-uniform-turnover", serde(default))]
    turnover: Turnover,

    #[serde(default)]
    #[serde(rename = "mode", alias = "loading_mode")]
    loading_mode: MapLoadingMode,
}

pub enum SpatiallyExplicitArgumentVariants {
    #[cfg(feature = "spatially-explicit-uniform-turnover")]
    UniformTurnover(uniform::SpatiallyExplicitUniformTurnoverArguments),
    #[cfg(feature = "spatially-explicit-turnover-map")]
    TurnoverMap(map::SpatiallyExplicitTurnoverMapArguments),
}

impl SpatiallyExplicitArguments {
    #[expect(clippy::missing_errors_doc)]
    pub fn try_load(self) -> Result<SpatiallyExplicitArgumentVariants, String> {
        match self {
            #[cfg(feature = "spatially-explicit-uniform-turnover")]
            Self {
                habitat_map,
                dispersal_map,
                turnover: Turnover::UniformRate(turnover_rate),
                loading_mode,
            } => uniform::SpatiallyExplicitUniformTurnoverArguments::try_load(
                habitat_map,
                dispersal_map,
                turnover_rate,
                loading_mode,
            )
            .map(SpatiallyExplicitArgumentVariants::UniformTurnover),
            #[cfg(feature = "spatially-explicit-turnover-map")]
            Self {
                habitat_map,
                dispersal_map,
                turnover: Turnover::Map(turnover_map),
                loading_mode,
            } => map::SpatiallyExplicitTurnoverMapArguments::try_load(
                habitat_map,
                dispersal_map,
                turnover_map,
                loading_mode,
            )
            .map(SpatiallyExplicitArgumentVariants::TurnoverMap),
        }
    }

    #[cfg(feature = "spatially-explicit-uniform-turnover")]
    #[must_use]
    pub fn from_uniform_rate(args: &uniform::SpatiallyExplicitUniformTurnoverArguments) -> Self {
        Self {
            habitat_map: args.habitat_path.clone(),
            dispersal_map: args.dispersal_path.clone(),
            turnover: Turnover::UniformRate(args.turnover_rate),
            loading_mode: args.loading_mode,
        }
    }

    #[cfg(feature = "spatially-explicit-turnover-map")]
    #[must_use]
    pub fn from_map(args: &map::SpatiallyExplicitTurnoverMapArguments) -> Self {
        Self {
            habitat_map: args.habitat_path.clone(),
            dispersal_map: args.dispersal_path.clone(),
            turnover: Turnover::Map(args.turnover_path.clone()),
            loading_mode: args.loading_mode,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Turnover {
    #[cfg(feature = "spatially-explicit-uniform-turnover")]
    #[serde(rename = "Uniform", alias = "Rate", alias = "UniformRate")]
    UniformRate(necsim_core_bond::PositiveF64),
    #[cfg(feature = "spatially-explicit-turnover-map")]
    Map(PathBuf),
}

#[cfg(feature = "spatially-explicit-uniform-turnover")]
impl Default for Turnover {
    fn default() -> Self {
        Self::UniformRate(necsim_core_bond::PositiveF64::new(0.5_f64).unwrap())
    }
}
