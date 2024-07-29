use necsim_core::{
    landscape::{IndexedLocation, Location},
    lineage::GlobalLineageReference,
};
use necsim_core_bond::PositiveF64;

use rusqlite::{named_params, types::Value};

use crate::{LastEventState, SpeciesIdentity};

use super::{IndividualSpeciesSQLiteReporter, SpeciesLocationsMode};

const METADATA_TABLE: &str = "__SPECIES_REPORTER_META";

impl IndividualSpeciesSQLiteReporter {
    pub(super) fn store_individual_origin(
        &mut self,
        lineage: &GlobalLineageReference,
        origin: &IndexedLocation,
    ) {
        self.origins.insert(lineage.clone(), origin.clone());
    }

    pub(super) fn store_individual_speciation(
        &mut self,
        lineage: &GlobalLineageReference,
        origin: &IndexedLocation,
        time: PositiveF64,
    ) {
        // Resolve the actual parent, irrespective of duplicate individuals
        let mut parent = lineage;
        while let Some(parent_parent) = self.parents.get(parent) {
            parent = parent_parent;
        }

        self.species.insert(
            parent.clone(),
            SpeciesIdentity::from_speciation(origin, time),
        );
    }

    pub(super) fn store_individual_coalescence(
        &mut self,
        child: &GlobalLineageReference,
        parent: &GlobalLineageReference,
    ) {
        // Resolve the actual child, irrespective of duplicate individuals
        let mut child = child;
        while let Some(child_parent) = self.parents.get(child) {
            child = child_parent;
        }
        let child = child.clone();

        // Resolve the actual parent, irrespective of duplicate individuals
        let mut parent = parent;
        while let Some(parent_parent) = self.parents.get(parent) {
            parent = parent_parent;
        }
        let parent = parent.clone();

        // Prevent a lookup-loop, can occur after `Resume`
        if child != parent {
            self.parents.insert(child, parent);
        }
    }

    #[expect(clippy::too_many_lines)]
    pub(super) fn initialise_sqlite_connection(&mut self) -> rusqlite::Result<()> {
        self.connection
            .pragma_update(None, "cache_size", self.cache.get())?;

        // Create the species locations table in `Create` mode
        if let SpeciesLocationsMode::Create = self.mode {
            self.connection.execute_batch(&format!(
                "CREATE TABLE {} (
                            id      INTEGER PRIMARY KEY NOT NULL,
                            x       INTEGER NOT NULL,
                            y       INTEGER NOT NULL,
                            i       INTEGER NOT NULL,
                            parent  INTEGER,
                            species TEXT
                        );
                        CREATE TABLE {METADATA_TABLE} (
                            key     TEXT PRIMARY KEY NOT NULL,
                            value   TEXT NOT NULL
                        );",
                self.table,
            ))?;
        }

        let mut schema: Vec<Vec<Value>> = Vec::new();

        // Collect the schema information for the species locations table
        self.connection
            .pragma(None, "table_info", &self.table, |row| {
                let mut schema_row = Vec::new();

                for col in 0..row.as_ref().column_count() {
                    schema_row.push(Value::from(row.get_ref(col)?));
                }

                schema.push(schema_row);

                Ok(())
            })?;

        // Check that the schema of the species locations table matches
        if schema
            != vec![
                vec![
                    Value::Integer(0),
                    Value::Text(String::from("id")),
                    Value::Text(String::from("INTEGER")),
                    Value::Integer(1),
                    Value::Null,
                    Value::Integer(1),
                ],
                vec![
                    Value::Integer(1),
                    Value::Text(String::from("x")),
                    Value::Text(String::from("INTEGER")),
                    Value::Integer(1),
                    Value::Null,
                    Value::Integer(0),
                ],
                vec![
                    Value::Integer(2),
                    Value::Text(String::from("y")),
                    Value::Text(String::from("INTEGER")),
                    Value::Integer(1),
                    Value::Null,
                    Value::Integer(0),
                ],
                vec![
                    Value::Integer(3),
                    Value::Text(String::from("i")),
                    Value::Text(String::from("INTEGER")),
                    Value::Integer(1),
                    Value::Null,
                    Value::Integer(0),
                ],
                vec![
                    Value::Integer(4),
                    Value::Text(String::from("parent")),
                    Value::Text(String::from("INTEGER")),
                    Value::Integer(0),
                    Value::Null,
                    Value::Integer(0),
                ],
                vec![
                    Value::Integer(5),
                    Value::Text(String::from("species")),
                    Value::Text(String::from("TEXT")),
                    Value::Integer(0),
                    Value::Null,
                    Value::Integer(0),
                ],
            ]
        {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ffi::ErrorCode::SchemaChanged,
                    extended_code: 0,
                },
                Some(format!(
                    "Invalid schema for the species locations table {}",
                    self.table
                )),
            ));
        }

        let mut schema: Vec<Vec<Value>> = Vec::new();

        // Collect the schema information for the metadata table
        self.connection
            .pragma(None, "table_info", METADATA_TABLE, |row| {
                let mut schema_row = Vec::new();

                for col in 0..row.as_ref().column_count() {
                    schema_row.push(Value::from(row.get_ref(col)?));
                }

                schema.push(schema_row);

                Ok(())
            })?;

        // Check that the schema of the metadata table matches
        if schema
            != vec![
                vec![
                    Value::Integer(0),
                    Value::Text(String::from("key")),
                    Value::Text(String::from("TEXT")),
                    Value::Integer(1),
                    Value::Null,
                    Value::Integer(1),
                ],
                vec![
                    Value::Integer(1),
                    Value::Text(String::from("value")),
                    Value::Text(String::from("TEXT")),
                    Value::Integer(1),
                    Value::Null,
                    Value::Integer(0),
                ],
            ]
        {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ffi::ErrorCode::SchemaChanged,
                    extended_code: 0,
                },
                Some(String::from(
                    "Invalid schema for the internal metadata table",
                )),
            ));
        }

        // Early return for the `Create` mode
        if let SpeciesLocationsMode::Create = self.mode {
            return Ok(());
        }

        let mut statement = self.connection.prepare(&format!(
            "SELECT id, x, y, i, parent, species FROM {}",
            self.table
        ))?;
        let mut query = statement.query([])?;

        // Resume from the existing species locations in the table
        while let Some(row) = query.next()? {
            let id: i64 = row.get("id")?;
            let x: i32 = row.get("x")?;
            let y: i32 = row.get("y")?;
            let i: i32 = row.get("i")?;

            let parent: Option<i64> = row.get("parent")?;
            let species: Option<String> = row.get("species")?;

            let id = unsafe { GlobalLineageReference::from_inner(from_i64(id)) };

            // Populate the individual `origins` lookup
            self.origins.insert(
                id.clone(),
                IndexedLocation::new(Location::new(from_i32(x), from_i32(y)), from_i32(i)),
            );

            if let Some(parent) = parent {
                let parent = unsafe { GlobalLineageReference::from_inner(from_i64(parent)) };

                // Populate the individual `parents` lookup
                self.parents.insert(id.clone(), parent);
            }

            if let Some(species) = species {
                let mut identity = [0_u8; 24];

                // Try to parse the species identity from its String form
                hex::decode_to_slice(&species, &mut identity).map_err(|_| {
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error {
                            code: rusqlite::ffi::ErrorCode::TypeMismatch,
                            extended_code: 0,
                        },
                        Some(format!(
                            "Invalid species identity {species:?} for individual #{id}",
                        )),
                    )
                })?;

                // Populate the individual `species` lookup
                self.species.insert(id, SpeciesIdentity::from(identity));
            }
        }

        let last_event: String = self
            .connection
            .query_row(
                &format!("SELECT value FROM {METADATA_TABLE} WHERE key='last-event'",),
                [],
                |row| row.get("value"),
            )
            .map_err(|_| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error {
                        code: rusqlite::ffi::ErrorCode::NotFound,
                        extended_code: 0,
                    },
                    Some(String::from("Failed to fetch the reporter resume metadata")),
                )
            })?;

        let LastEventState {
            last_parent_prior_time,
            last_speciation_event,
            last_dispersal_event,
        } = LastEventState::from_string(&last_event).map_err(|()| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ffi::ErrorCode::TypeMismatch,
                    extended_code: 0,
                },
                Some(String::from(
                    "Failed to decode the reporter resume metadata",
                )),
            )
        })?;

        self.last_parent_prior_time = last_parent_prior_time;
        self.last_speciation_event = last_speciation_event;
        self.last_dispersal_event = last_dispersal_event;

        Ok(())
    }

    pub(super) fn output_to_database(mut self) -> rusqlite::Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)?;

        let mut insertion = tx.prepare(&format!(
            "INSERT OR REPLACE INTO {} VALUES (:id, :x, :y, :i, :parent, :species)",
            self.table,
        ))?;

        // Lineage ancestor union-find with path compression
        let mut ancestors = self.parents.clone();

        let mut family = Vec::new();

        for (lineage, origin) in self.origins {
            // Find the ancestor that originated the species
            let mut ancestor = lineage.clone();
            while let Some(ancestor_parent) = ancestors.get(&ancestor) {
                family.push(ancestor.clone());
                ancestor = ancestor_parent.clone();
            }

            // Compress the ancestry paths for all visited lineages
            for child in family.drain(..) {
                ancestors.insert(child, ancestor.clone());
            }

            // Positional parameters boost performance
            insertion.execute(rusqlite::params![
                /* :id */ to_i64(unsafe { lineage.clone().into_inner() }),
                /* :x */ to_i32(origin.location().x()),
                /* :y */ to_i32(origin.location().y()),
                /* :i */ to_i32(origin.index()),
                /* :parent */
                self.parents
                    .get(&lineage)
                    .map(|parent| to_i64(unsafe { parent.clone().into_inner() })),
                /* :species */
                self.species
                    .get(&ancestor)
                    .map(|species| hex::encode(**species)),
            ])?;
        }

        insertion.finalize()?;

        let last_event_state = LastEventState {
            last_parent_prior_time: self.last_parent_prior_time,
            last_speciation_event: self.last_speciation_event,
            last_dispersal_event: self.last_dispersal_event,
        }
        .into_string()
        .map_err(|()| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ffi::ErrorCode::TypeMismatch,
                    extended_code: 0,
                },
                Some(String::from(
                    "Failed to encode the reporter resume metadata",
                )),
            )
        })?;

        tx.execute(
            &format!("INSERT OR REPLACE INTO {METADATA_TABLE} VALUES (:key, :value)",),
            named_params! { ":key": "last-event", ":value": last_event_state },
        )?;

        tx.commit()?;

        self.connection.close().map_err(|(_, err)| err)
    }
}

const fn to_i32(x: u32) -> i32 {
    i32::from_ne_bytes(x.to_ne_bytes())
}

const fn to_i64(x: u64) -> i64 {
    i64::from_ne_bytes(x.to_ne_bytes())
}

const fn from_i32(x: i32) -> u32 {
    u32::from_ne_bytes(x.to_ne_bytes())
}

const fn from_i64(x: i64) -> u64 {
    u64::from_ne_bytes(x.to_ne_bytes())
}
