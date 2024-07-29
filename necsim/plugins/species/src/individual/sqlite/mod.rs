use std::{collections::HashMap, fmt, num::NonZeroI32, path::PathBuf};

use fnv::FnvBuildHasher;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Deserializer, Serialize};

use necsim_core::{
    event::{DispersalEvent, SpeciationEvent},
    landscape::IndexedLocation,
    lineage::GlobalLineageReference,
};
use necsim_core_bond::NonNegativeF64;

use crate::SpeciesIdentity;

mod database;
mod reporter;

pub struct IndividualSpeciesSQLiteReporter {
    last_parent_prior_time: Option<(GlobalLineageReference, NonNegativeF64)>,
    last_speciation_event: Option<SpeciationEvent>,
    last_dispersal_event: Option<DispersalEvent>,

    // Original (present-time) locations of all lineages
    origins: HashMap<GlobalLineageReference, IndexedLocation, FnvBuildHasher>,
    // Child -> Parent lineage mapping
    parents: HashMap<GlobalLineageReference, GlobalLineageReference, FnvBuildHasher>,
    // Species originator -> Species identities mapping
    species: HashMap<GlobalLineageReference, SpeciesIdentity, FnvBuildHasher>,

    output: PathBuf,
    table: String,
    mode: SpeciesLocationsMode,
    cache: NonZeroI32,

    connection: Connection,
}

impl fmt::Debug for IndividualSpeciesSQLiteReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(IndividualSpeciesSQLiteReporter))
            .field("output", &self.output)
            .field("table", &self.table)
            .field("mode", &self.mode)
            .field("cache", &self.cache)
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for IndividualSpeciesSQLiteReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        IndividualSpeciesSQLiteReporterArgs {
            output: self.output.clone(),
            table: self.table.clone(),
            mode: self.mode.clone(),
            cache: self.cache,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for IndividualSpeciesSQLiteReporter {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let args = IndividualSpeciesSQLiteReporterArgs::deserialize(deserializer)?;

        let connection = Connection::open_with_flags(
            &args.output,
            match args.mode {
                SpeciesLocationsMode::Resume => OpenFlags::SQLITE_OPEN_READ_WRITE,
                SpeciesLocationsMode::Create => {
                    OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE
                },
            },
        )
        .map_err(serde::de::Error::custom)?;

        Ok(Self {
            last_parent_prior_time: None,
            last_speciation_event: None,
            last_dispersal_event: None,

            origins: HashMap::default(),
            parents: HashMap::default(),
            species: HashMap::default(),

            output: args.output,
            table: args.table,
            mode: args.mode,
            cache: args.cache,

            connection,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "IndividualSpeciesSQLiteReporter")]
struct IndividualSpeciesSQLiteReporterArgs {
    output: PathBuf,
    #[serde(default = "default_table_name")]
    table: String,
    #[serde(default)]
    mode: SpeciesLocationsMode,
    #[serde(default = "default_cache_size")]
    cache: NonZeroI32,
}

fn default_table_name() -> String {
    String::from("SPECIES_LOCATIONS")
}

fn default_cache_size() -> NonZeroI32 {
    NonZeroI32::new(1_000_000_i32).unwrap()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
enum SpeciesLocationsMode {
    #[default]
    Create,
    Resume,
}
