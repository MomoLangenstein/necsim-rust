use necsim_impls_no_std::cogs::{
    habitat::almost_infinite::AlmostInfiniteHabitat,
    origin_sampler::{
        pre_sampler::OriginPreSampler,
        singleton_demes::{
            circle::SingletonDemesCircleOriginSampler,
            rectangle::SingletonDemesRectangleOriginSampler, SingletonDemesOriginSampler,
        },
    },
};
use serde::{Deserialize, Serialize};

use necsim_core::{
    cogs::MathsCore,
    landscape::{LandscapeExtent, Location},
};
#[cfg(feature = "almost-infinite-normal-dispersal")]
use necsim_core_bond::NonNegativeF64;
#[cfg(feature = "almost-infinite-clark2dt-dispersal")]
use necsim_core_bond::PositiveF64;

#[cfg(feature = "almost-infinite-clark2dt-dispersal")]
pub mod clark2dt;
#[cfg(feature = "almost-infinite-downscaled")]
pub mod downscaled;
#[cfg(feature = "almost-infinite-normal-dispersal")]
pub mod normal;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "AlmostInfinite")]
pub struct AlmostInfiniteArguments {
    sample: Sample,
    dispersal: Dispersal,
    #[cfg(feature = "almost-infinite-downscaled")]
    #[serde(default)]
    downscale: Option<downscaled::Downscale>,
}

#[expect(clippy::module_name_repetitions)]
pub enum AlmostInfiniteArgumentVariants {
    #[cfg(feature = "almost-infinite-normal-dispersal")]
    Normal(normal::AlmostInfiniteNormalDispersalArguments),
    #[cfg(feature = "almost-infinite-clark2dt-dispersal")]
    Clark2Dt(clark2dt::AlmostInfiniteClark2DtDispersalArguments),
    #[cfg(all(
        feature = "almost-infinite-downscaled",
        feature = "almost-infinite-normal-dispersal"
    ))]
    DownscaledNormal(
        downscaled::AlmostInfiniteDownscaledArguments<
            normal::AlmostInfiniteNormalDispersalScenario,
        >,
    ),
    #[cfg(all(
        feature = "almost-infinite-downscaled",
        feature = "almost-infinite-clark2dt-dispersal"
    ))]
    DownscaledClark2Dt(
        downscaled::AlmostInfiniteDownscaledArguments<
            clark2dt::AlmostInfiniteClark2DtDispersalScenario,
        >,
    ),
}

impl AlmostInfiniteArguments {
    #[must_use]
    pub fn load(self) -> AlmostInfiniteArgumentVariants {
        match self {
            #[cfg(feature = "almost-infinite-normal-dispersal")]
            Self {
                sample,
                dispersal: Dispersal::Normal { sigma },
                #[cfg(feature = "almost-infinite-downscaled")]
                    downscale: None,
            } => AlmostInfiniteArgumentVariants::Normal(
                normal::AlmostInfiniteNormalDispersalArguments { sample, sigma },
            ),
            #[cfg(feature = "almost-infinite-clark2dt-dispersal")]
            Self {
                sample,
                dispersal: Dispersal::Clark2Dt { shape_u, tail_p },
                #[cfg(feature = "almost-infinite-downscaled")]
                    downscale: None,
            } => AlmostInfiniteArgumentVariants::Clark2Dt(
                clark2dt::AlmostInfiniteClark2DtDispersalArguments {
                    sample,
                    shape_u,
                    tail_p,
                },
            ),
            #[cfg(all(
                feature = "almost-infinite-downscaled",
                feature = "almost-infinite-normal-dispersal"
            ))]
            Self {
                sample,
                dispersal: Dispersal::Normal { sigma },
                downscale: Some(downscale),
            } => AlmostInfiniteArgumentVariants::DownscaledNormal(
                downscaled::AlmostInfiniteDownscaledArguments {
                    args: normal::AlmostInfiniteNormalDispersalArguments { sample, sigma },
                    downscale,
                },
            ),
            #[cfg(all(
                feature = "almost-infinite-downscaled",
                feature = "almost-infinite-clark2dt-dispersal"
            ))]
            Self {
                sample,
                dispersal: Dispersal::Clark2Dt { shape_u, tail_p },
                downscale: Some(downscale),
            } => AlmostInfiniteArgumentVariants::DownscaledClark2Dt(
                downscaled::AlmostInfiniteDownscaledArguments {
                    args: clark2dt::AlmostInfiniteClark2DtDispersalArguments {
                        sample,
                        shape_u,
                        tail_p,
                    },
                    downscale,
                },
            ),
        }
    }

    #[cfg(feature = "almost-infinite-normal-dispersal")]
    #[must_use]
    pub fn from_normal(args: &normal::AlmostInfiniteNormalDispersalArguments) -> Self {
        Self {
            sample: args.sample.clone(),
            dispersal: Dispersal::Normal { sigma: args.sigma },
            #[cfg(feature = "almost-infinite-downscaled")]
            downscale: None,
        }
    }

    #[cfg(feature = "almost-infinite-clark2dt-dispersal")]
    #[must_use]
    pub fn from_clark2dt(args: &clark2dt::AlmostInfiniteClark2DtDispersalArguments) -> Self {
        Self {
            sample: args.sample.clone(),
            dispersal: Dispersal::Clark2Dt {
                shape_u: args.shape_u,
                tail_p: args.tail_p,
            },
            #[cfg(feature = "almost-infinite-downscaled")]
            downscale: None,
        }
    }

    #[cfg(all(
        feature = "almost-infinite-downscaled",
        feature = "almost-infinite-normal-dispersal"
    ))]
    #[must_use]
    pub fn from_downscaled_normal(
        args: &downscaled::AlmostInfiniteDownscaledArguments<
            normal::AlmostInfiniteNormalDispersalScenario,
        >,
    ) -> Self {
        Self {
            sample: args.args.sample.clone(),
            dispersal: Dispersal::Normal {
                sigma: args.args.sigma,
            },
            downscale: Some(args.downscale.clone()),
        }
    }

    #[cfg(all(
        feature = "almost-infinite-downscaled",
        feature = "almost-infinite-clark2dt-dispersal"
    ))]
    #[must_use]
    pub fn from_downscaled_clark2dt(
        args: &downscaled::AlmostInfiniteDownscaledArguments<
            clark2dt::AlmostInfiniteClark2DtDispersalScenario,
        >,
    ) -> Self {
        Self {
            sample: args.args.sample.clone(),
            dispersal: Dispersal::Clark2Dt {
                shape_u: args.args.shape_u,
                tail_p: args.args.tail_p,
            },
            downscale: Some(args.downscale.clone()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Sample {
    Circle {
        #[serde(default = "Sample::default_circle_sample_centre")]
        centre: Location,
        radius: u16,
    },
    Rectangle(LandscapeExtent),
}

impl Sample {
    #[must_use]
    pub const fn default_circle_sample_centre() -> Location {
        const HABITAT_CENTRE: u32 = u32::MAX / 2;

        Location::new(HABITAT_CENTRE, HABITAT_CENTRE)
    }

    pub fn into_origin_sampler<M: MathsCore, I: Iterator<Item = u64>>(
        self,
        habitat: &AlmostInfiniteHabitat<M>,
        pre_sampler: OriginPreSampler<M, I>,
    ) -> SingletonDemesOriginSampler<M, AlmostInfiniteHabitat<M>, I> {
        match self {
            Self::Circle { centre, radius } => SingletonDemesOriginSampler::Circle(
                SingletonDemesCircleOriginSampler::new(pre_sampler, habitat, centre, radius),
            ),
            Self::Rectangle(sample) => SingletonDemesOriginSampler::Rectangle(
                SingletonDemesRectangleOriginSampler::new(pre_sampler, habitat, sample),
            ),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Dispersal {
    #[cfg(feature = "almost-infinite-normal-dispersal")]
    #[serde(alias = "Gaussian")]
    Normal { sigma: NonNegativeF64 },
    #[cfg(feature = "almost-infinite-clark2dt-dispersal")]
    Clark2Dt {
        #[serde(alias = "u")]
        shape_u: PositiveF64,
        #[serde(default = "PositiveF64::one")]
        #[serde(alias = "p")]
        tail_p: PositiveF64,
    },
}
