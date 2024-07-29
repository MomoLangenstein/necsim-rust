use serde::{Deserialize, Deserializer, Serialize};
use serde_state::DeserializeState;

use necsim_core::lineage::Lineage;
use necsim_core_bond::{ClosedUnitF64, NonNegativeF64, PositiveF64};
use necsim_impls_std::lineage_file::loader::LineageFileLoader;

use rustcoalescence_algorithms::strategy::RestartFixUpStrategy;

use super::pause::{Pause, PauseMode};

mod origin;

#[derive(Clone, Debug, Serialize)]
pub struct Sample {
    pub percentage: ClosedUnitF64,
    pub origin: SampleOrigin,
    pub mode: SampleMode,
}

impl Default for Sample {
    fn default() -> Self {
        let raw = SampleRaw::default();

        Self {
            percentage: raw.percentage,
            origin: raw.origin,
            mode: raw.mode,
        }
    }
}

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, Serialize)]
pub enum SampleOrigin {
    Habitat,
    List(Vec<Lineage>),
    Bincode(LineageFileLoader),
}

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SampleMode {
    Genesis,
    Resume,
    FixUp(RestartFixUpStrategy),
    Restart(SampleModeRestart),
}

impl Default for SampleMode {
    fn default() -> Self {
        Self::Genesis
    }
}

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SampleModeRestart {
    pub after: NonNegativeF64,
}

impl<'de> DeserializeState<'de, &'de Option<Pause>> for Sample {
    fn deserialize_state<D: Deserializer<'de>>(
        pause: &mut &'de Option<Pause>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let raw = SampleRaw::deserialize_state(pause, deserializer)?;

        match (&raw.origin, &raw.mode) {
            (SampleOrigin::Habitat, SampleMode::Genesis)
            | (
                SampleOrigin::List(_) | SampleOrigin::Bincode(_),
                SampleMode::Resume | SampleMode::FixUp(_) | SampleMode::Restart(_),
            ) => (),
            (
                SampleOrigin::Habitat,
                SampleMode::Resume | SampleMode::FixUp(_) | SampleMode::Restart(_),
            ) => {
                return Err(serde::de::Error::custom(
                    "`Habitat` origin is only compatible with `Genesis` mode",
                ));
            },
            (SampleOrigin::List(_) | SampleOrigin::Bincode(_), SampleMode::Genesis) => {
                return Err(serde::de::Error::custom(
                    "`Genesis` mode is only compatible with `Habitat` origin",
                ));
            },
        }

        let pre_resume_bound = match &raw.mode {
            SampleMode::Genesis | SampleMode::Resume => None,
            SampleMode::FixUp(_) => {
                if let Some(pause) = pause {
                    match pause.mode {
                        PauseMode::Resume => {
                            return Err(serde::de::Error::custom(
                                "`FixUp` sample mode is incompatible with `Resume` pause mode,\n \
                                 use `Restart` instead",
                            ))
                        },
                        PauseMode::FixUp | PauseMode::Restart => (),
                    }

                    match PositiveF64::new(pause.before.get()) {
                        Ok(fix_at) => Some(fix_at),
                        Err(_) => {
                            return Err(serde::de::Error::custom(
                                "`FixUp` mode cannot be used at simulation genesis time 0.0",
                            ))
                        },
                    }
                } else {
                    return Err(serde::de::Error::custom(
                        "`FixUp` mode requires an immediate pause to save the fixed individuals",
                    ));
                }
            },
            SampleMode::Restart(SampleModeRestart { after }) => {
                Some(PositiveF64::max_after(*after, *after))
            },
        };

        let lineages = match &raw.origin {
            SampleOrigin::Habitat => None,
            SampleOrigin::List(lineages) => Some(lineages.iter()),
            SampleOrigin::Bincode(loader) => Some(loader.get_lineages().iter()),
        };

        if let (Some(lineages), Some(pre_resume_bound)) = (lineages, pre_resume_bound) {
            for lineage in lineages {
                if lineage.last_event_time >= pre_resume_bound {
                    return Err(serde::de::Error::custom(format!(
                        "Lineage #{} at time {} is not before the resume point",
                        lineage.global_reference, lineage.last_event_time
                    )));
                }
            }
        }

        Ok(Self {
            percentage: raw.percentage,
            origin: raw.origin,
            mode: raw.mode,
        })
    }
}

#[derive(Debug, DeserializeState)]
#[serde(deserialize_state = "&'de Option<Pause>")]
#[serde(default)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Sample")]
struct SampleRaw {
    percentage: ClosedUnitF64,
    #[serde(deserialize_state)]
    origin: SampleOrigin,
    mode: SampleMode,
}

impl Default for SampleRaw {
    fn default() -> Self {
        Self {
            percentage: ClosedUnitF64::one(),
            origin: SampleOrigin::Habitat,
            mode: SampleMode::Genesis,
        }
    }
}
