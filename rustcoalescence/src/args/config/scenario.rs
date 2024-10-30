use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug)]
pub enum Scenario {
    #[cfg(feature = "spatially-explicit-uniform-turnover-scenario")]
    SpatiallyExplicitUniformTurnover(
        rustcoalescence_scenarios::spatially_explicit::uniform::SpatiallyExplicitUniformTurnoverArguments,
    ),
    #[cfg(feature = "spatially-explicit-turnover-map-scenario")]
    SpatiallyExplicitTurnoverMap(
        rustcoalescence_scenarios::spatially_explicit::map::SpatiallyExplicitTurnoverMapArguments,
    ),
    #[cfg(feature = "non-spatial-scenario")]
    NonSpatial(rustcoalescence_scenarios::non_spatial::NonSpatialArguments),
    #[cfg(feature = "spatially-implicit-scenario")]
    SpatiallyImplicit(rustcoalescence_scenarios::spatially_implicit::SpatiallyImplicitArguments),
    #[cfg(feature = "almost-infinite-normal-dispersal-scenario")]
    AlmostInfiniteNormalDispersal(rustcoalescence_scenarios::almost_infinite::normal::AlmostInfiniteNormalDispersalArguments),
    #[cfg(feature = "almost-infinite-clark2dt-dispersal-scenario")]
    AlmostInfiniteClark2DtDispersal(rustcoalescence_scenarios::almost_infinite::clark2dt::AlmostInfiniteClark2DtDispersalArguments),
    #[cfg(all(
        feature = "almost-infinite-normal-dispersal-scenario",
        feature = "almost-infinite-downscaled-scenario",
    ))]
    AlmostInfiniteDownscaledNormalDispersal(rustcoalescence_scenarios::almost_infinite::downscaled::AlmostInfiniteDownscaledArguments<rustcoalescence_scenarios::almost_infinite::normal::AlmostInfiniteNormalDispersalScenario>),
    #[cfg(all(
        feature = "almost-infinite-clark2dt-dispersal-scenario",
        feature = "almost-infinite-downscaled-scenario",
    ))]
    AlmostInfiniteDownscaledClark2DtDispersal(rustcoalescence_scenarios::almost_infinite::downscaled::AlmostInfiniteDownscaledArguments<rustcoalescence_scenarios::almost_infinite::clark2dt::AlmostInfiniteClark2DtDispersalScenario>),
    #[cfg(feature = "wrapping-noise-scenario")]
    WrappingNoise(rustcoalescence_scenarios::wrapping_noise::WrappingNoiseArguments),
}

impl Serialize for Scenario {
    #[allow(unused_variables)] // FIXME: use expect
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let scenario: ScenarioRaw = match *self {
            #[cfg(feature = "spatially-explicit-uniform-turnover-scenario")]
            Self::SpatiallyExplicitUniformTurnover(ref args) => ScenarioRaw::SpatiallyExplicit(
                rustcoalescence_scenarios::spatially_explicit::SpatiallyExplicitArguments::from_uniform_rate(args),
            ),
            #[cfg(feature = "spatially-explicit-turnover-map-scenario")]
            Self::SpatiallyExplicitTurnoverMap(ref args) => ScenarioRaw::SpatiallyExplicit(
                rustcoalescence_scenarios::spatially_explicit::SpatiallyExplicitArguments::from_map(args),
            ),
            #[cfg(feature = "non-spatial-scenario")]
            Self::NonSpatial(ref args) => ScenarioRaw::NonSpatial(args.clone()),
            #[cfg(feature = "spatially-implicit-scenario")]
            Self::SpatiallyImplicit(ref args) => ScenarioRaw::SpatiallyImplicit(args.clone()),
            #[cfg(feature = "almost-infinite-normal-dispersal-scenario")]
            Self::AlmostInfiniteNormalDispersal(ref args) => ScenarioRaw::AlmostInfinite(
                rustcoalescence_scenarios::almost_infinite::AlmostInfiniteArguments::from_normal(args),
            ),
            #[cfg(feature = "almost-infinite-clark2dt-dispersal-scenario")]
            Self::AlmostInfiniteClark2DtDispersal(ref args) => ScenarioRaw::AlmostInfinite(
                rustcoalescence_scenarios::almost_infinite::AlmostInfiniteArguments::from_clark2dt(args),
            ),
            #[cfg(all(
                feature = "almost-infinite-normal-dispersal-scenario",
                feature = "almost-infinite-downscaled-scenario",
            ))]
            Self::AlmostInfiniteDownscaledNormalDispersal(ref args) => ScenarioRaw::AlmostInfinite(
                rustcoalescence_scenarios::almost_infinite::AlmostInfiniteArguments::from_downscaled_normal(args),
            ),
            #[cfg(all(
                feature = "almost-infinite-clark2dt-dispersal-scenario",
                feature = "almost-infinite-downscaled-scenario",
            ))]
            Self::AlmostInfiniteDownscaledClark2DtDispersal(ref args) => ScenarioRaw::AlmostInfinite(
                rustcoalescence_scenarios::almost_infinite::AlmostInfiniteArguments::from_downscaled_clark2dt(args),
            ),
            #[cfg(feature = "wrapping-noise-scenario")]
            Self::WrappingNoise(ref args) => ScenarioRaw::WrappingNoise(args.clone()),
        };

        #[allow(unreachable_code)] // FIXME: use expect
        scenario.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Scenario {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match ScenarioRaw::deserialize(deserializer)? {
            #[cfg(any(
                feature = "spatially-explicit-uniform-turnover-scenario",
                feature = "spatially-explicit-turnover-map-scenario",
            ))]
            ScenarioRaw::SpatiallyExplicit(args) => match args.try_load().map_err(serde::de::Error::custom)? {
                #[cfg(feature = "spatially-explicit-uniform-turnover-scenario")]
                rustcoalescence_scenarios::spatially_explicit::SpatiallyExplicitArgumentVariants::UniformTurnover(args) => Ok(Self::SpatiallyExplicitUniformTurnover(args)),
                #[cfg(feature = "spatially-explicit-turnover-map-scenario")]
                rustcoalescence_scenarios::spatially_explicit::SpatiallyExplicitArgumentVariants::TurnoverMap(args) => Ok(Self::SpatiallyExplicitTurnoverMap(args)),
            },
            #[cfg(feature = "non-spatial-scenario")]
            ScenarioRaw::NonSpatial(args) => Ok(Self::NonSpatial(args)),
            #[cfg(feature = "spatially-implicit-scenario")]
            ScenarioRaw::SpatiallyImplicit(args) => Ok(Self::SpatiallyImplicit(args)),
            #[cfg(any(
                feature = "almost-infinite-normal-dispersal-scenario",
                feature = "almost-infinite-clark2dt-dispersal-scenario",
            ))]
            ScenarioRaw::AlmostInfinite(args) => match args.load() {
                #[cfg(feature = "almost-infinite-normal-dispersal-scenario")]
                rustcoalescence_scenarios::almost_infinite::AlmostInfiniteArgumentVariants::Normal(args) => Ok(Self::AlmostInfiniteNormalDispersal(args)),
                #[cfg(feature = "almost-infinite-clark2dt-dispersal-scenario")]
                rustcoalescence_scenarios::almost_infinite::AlmostInfiniteArgumentVariants::Clark2Dt(args) => Ok(Self::AlmostInfiniteClark2DtDispersal(args)),
                #[cfg(all(
                    feature = "almost-infinite-normal-dispersal-scenario",
                    feature = "almost-infinite-downscaled-scenario",
                ))]
                rustcoalescence_scenarios::almost_infinite::AlmostInfiniteArgumentVariants::DownscaledNormal(args) => Ok(Self::AlmostInfiniteDownscaledNormalDispersal(args)),
                #[cfg(all(
                    feature = "almost-infinite-clark2dt-dispersal-scenario",
                    feature = "almost-infinite-downscaled-scenario",
                ))]
                rustcoalescence_scenarios::almost_infinite::AlmostInfiniteArgumentVariants::DownscaledClark2Dt(args) => Ok(Self::AlmostInfiniteDownscaledClark2DtDispersal(args)),
            },
            #[cfg(feature = "wrapping-noise-scenario")]
            ScenarioRaw::WrappingNoise(args) => Ok(Self::WrappingNoise(args)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "Scenario")]
enum ScenarioRaw {
    #[cfg(any(
        feature = "spatially-explicit-uniform-turnover-scenario",
        feature = "spatially-explicit-turnover-map-scenario",
    ))]
    SpatiallyExplicit(rustcoalescence_scenarios::spatially_explicit::SpatiallyExplicitArguments),
    #[cfg(feature = "non-spatial-scenario")]
    NonSpatial(rustcoalescence_scenarios::non_spatial::NonSpatialArguments),
    #[cfg(feature = "spatially-implicit-scenario")]
    SpatiallyImplicit(rustcoalescence_scenarios::spatially_implicit::SpatiallyImplicitArguments),
    #[cfg(any(
        feature = "almost-infinite-normal-dispersal-scenario",
        feature = "almost-infinite-clark2dt-dispersal-scenario",
    ))]
    AlmostInfinite(rustcoalescence_scenarios::almost_infinite::AlmostInfiniteArguments),
    #[cfg(feature = "wrapping-noise-scenario")]
    WrappingNoise(rustcoalescence_scenarios::wrapping_noise::WrappingNoiseArguments),
}
