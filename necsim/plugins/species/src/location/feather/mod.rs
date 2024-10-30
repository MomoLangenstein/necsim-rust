use std::{collections::HashMap, convert::TryFrom, fmt, fs::File, io::BufReader, path::PathBuf};

use arrow2::{
    array::{FixedSizeBinaryArray, PrimitiveArray},
    datatypes::{DataType, Field},
};
use fnv::FnvBuildHasher;
use serde::{Deserialize, Deserializer, Serialize};

use necsim_core::{
    event::{DispersalEvent, SpeciationEvent},
    landscape::Location,
    lineage::GlobalLineageReference,
};
use necsim_core_bond::{ClosedUnitF64, NonNegativeF64, PositiveF64};

use crate::{LastEventState, SpeciesIdentity};

mod dataframe;
mod reporter;

pub struct LocationSpeciesFeatherReporter {
    last_parent_prior_time: Option<(GlobalLineageReference, NonNegativeF64)>,
    last_speciation_event: Option<SpeciationEvent>,
    last_dispersal_event: Option<DispersalEvent>,

    // Representation counts for all resumed lineages
    counts: HashMap<GlobalLineageReference, u64, FnvBuildHasher>,
    // Last event time of all lineages
    activity: HashMap<GlobalLineageReference, PositiveF64, FnvBuildHasher>,
    // Original (present-time) locations of all lineages
    origins: HashMap<GlobalLineageReference, Location, FnvBuildHasher>,

    // Child -> Parent lineage mapping
    parents: HashMap<GlobalLineageReference, GlobalLineageReference, FnvBuildHasher>,

    // Species originator -> Species identity mapping
    species: HashMap<GlobalLineageReference, SpeciesIdentity, FnvBuildHasher>,
    // All speciated location-species records from before a resume
    speciated: Vec<(Location, SpeciesIdentity, u64)>,

    output: PathBuf,
    deduplication_probability: ClosedUnitF64,
    mode: SpeciesLocationsMode,
    init: bool,
}

impl Drop for LocationSpeciesFeatherReporter {
    fn drop(&mut self) {
        if matches!(self.mode, SpeciesLocationsMode::Create) && !self.init {
            std::mem::drop(std::fs::remove_file(&self.output));
        }
    }
}

impl fmt::Debug for LocationSpeciesFeatherReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(LocationSpeciesFeatherReporter))
            .field("output", &self.output)
            .field(
                "deduplication",
                &SpeciesDeduplicationMode::from(self.deduplication_probability),
            )
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for LocationSpeciesFeatherReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        LocationSpeciesFeatherReporterArgs {
            output: self.output.clone(),
            deduplication: SpeciesDeduplicationMode::from(self.deduplication_probability),
            mode: self.mode.clone(),
        }
        .serialize(serializer)
    }
}

#[expect(clippy::too_many_lines)]
impl<'de> Deserialize<'de> for LocationSpeciesFeatherReporter {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let args = LocationSpeciesFeatherReporterArgs::deserialize(deserializer)?;

        let mut self_last_parent_prior_time = None;
        let mut self_last_speciation_event = None;
        let mut self_last_dispersal_event = None;

        let mut self_counts = HashMap::default();
        let mut self_origins = HashMap::default();

        let mut self_parents = HashMap::default();

        let mut self_speciated = Vec::default();

        if matches!(args.mode, SpeciesLocationsMode::Resume) {
            let file = File::options()
                .read(true)
                .open(&args.output)
                .map_err(serde::de::Error::custom)?;
            let mut reader = BufReader::new(file);

            let metadata = arrow2::io::ipc::read::read_file_metadata(&mut reader)
                .map_err(serde::de::Error::custom)?;

            let expected_fields = vec![
                Field::new("x", DataType::UInt32, false),
                Field::new("y", DataType::UInt32, false),
                Field::new("species", DataType::FixedSizeBinary(24), false),
                Field::new("count", DataType::UInt64, false),
            ];

            if metadata.schema.fields != expected_fields {
                return Err(serde::de::Error::custom(
                    "species dataframe schema mismatch",
                ));
            }

            let last_event = match metadata.schema.metadata.get("last-event") {
                Some(last_event) => LastEventState::from_string(last_event).map_err(|()| {
                    serde::de::Error::custom("invalid resume metadata in species dataframe")
                })?,
                None => {
                    return Err(serde::de::Error::custom(
                        "resume metadata missing from species dataframe",
                    ))
                },
            };

            self_last_parent_prior_time = last_event.last_parent_prior_time;
            self_last_speciation_event = last_event.last_speciation_event;
            self_last_dispersal_event = last_event.last_dispersal_event;

            for chunk in arrow2::io::ipc::read::FileReader::new(reader, metadata, None, None) {
                let chunk = chunk.map_err(serde::de::Error::custom)?;

                let [xs, ys, species, counts] = chunk.columns() else {
                    return Err(serde::de::Error::custom(
                        "corrupted species dataframe schema",
                    ));
                };

                let Some(xs) = xs.as_any().downcast_ref::<PrimitiveArray<u32>>() else {
                    return Err(serde::de::Error::custom(
                        "corrupted species dataframe x column",
                    ));
                };

                let Some(ys) = ys.as_any().downcast_ref::<PrimitiveArray<u32>>() else {
                    return Err(serde::de::Error::custom(
                        "corrupted species dataframe y column",
                    ));
                };

                let species = match species.as_any().downcast_ref::<FixedSizeBinaryArray>() {
                    Some(species) if species.size() == 24 => species,
                    _ => {
                        return Err(serde::de::Error::custom(
                            "corrupted species dataframe species column",
                        ))
                    },
                };

                let Some(counts) = counts.as_any().downcast_ref::<PrimitiveArray<u64>>() else {
                    return Err(serde::de::Error::custom(
                        "corrupted species dataframe count column",
                    ));
                };

                for (((x, y), species), count) in xs
                    .values_iter()
                    .zip(ys.values_iter())
                    .zip(species.values_iter())
                    .zip(counts.values_iter())
                {
                    let origin = Location::new(*x, *y);
                    let species = SpeciesIdentity::try_from(species).map_err(|_| {
                        serde::de::Error::custom("corrupted species dataframe species value")
                    })?;
                    let count = *count;

                    match species.try_into_unspeciated() {
                        Ok((lineage, anchor)) => {
                            if count != 1 {
                                self_counts.insert(lineage.clone(), count);
                            }

                            self_origins.insert(lineage.clone(), origin);

                            if lineage != anchor {
                                self_parents.insert(lineage, anchor);
                            }
                        },
                        Err(species) => {
                            self_speciated.push((origin, species, count));
                        },
                    }
                }
            }
        } else {
            File::options()
                .create_new(true)
                .write(true)
                .open(&args.output)
                .map_err(serde::de::Error::custom)?;
        }

        Ok(Self {
            last_parent_prior_time: self_last_parent_prior_time,
            last_speciation_event: self_last_speciation_event,
            last_dispersal_event: self_last_dispersal_event,

            counts: self_counts,
            activity: HashMap::default(),
            origins: self_origins,

            parents: self_parents,

            species: HashMap::default(),
            speciated: self_speciated,

            deduplication_probability: match args.deduplication {
                SpeciesDeduplicationMode::None => ClosedUnitF64::zero(),
                SpeciesDeduplicationMode::Fixed(SpeciesDeduplicationLevel { level }) => level,
                SpeciesDeduplicationMode::Full => ClosedUnitF64::one(),
            },

            output: args.output,
            mode: args.mode,
            init: false,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "LocationSpeciesFeatherReporter")]
struct LocationSpeciesFeatherReporterArgs {
    output: PathBuf,
    #[serde(default)]
    deduplication: SpeciesDeduplicationMode,
    #[serde(default)]
    mode: SpeciesLocationsMode,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
enum SpeciesLocationsMode {
    #[default]
    Create,
    Resume,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum SpeciesDeduplicationMode {
    Full,
    None,
    Fixed(SpeciesDeduplicationLevel),
}

impl Default for SpeciesDeduplicationMode {
    fn default() -> Self {
        Self::Fixed(SpeciesDeduplicationLevel {
            level: ClosedUnitF64::new(0.0625_f64).unwrap(),
        })
    }
}

impl From<ClosedUnitF64> for SpeciesDeduplicationMode {
    fn from(level: ClosedUnitF64) -> Self {
        if level == ClosedUnitF64::zero() {
            Self::None
        } else if level == ClosedUnitF64::one() {
            Self::Full
        } else {
            Self::Fixed(SpeciesDeduplicationLevel { level })
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SpeciesDeduplicationLevel {
    level: ClosedUnitF64,
}
