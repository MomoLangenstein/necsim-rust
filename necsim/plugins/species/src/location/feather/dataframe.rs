use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap},
    fs::File,
    io::BufWriter,
};

use arrow2::{
    array::{FixedSizeBinaryArray, PrimitiveArray},
    buffer::Buffer,
    chunk::Chunk,
    datatypes::{DataType, Field, Schema},
    io::ipc::write::{FileWriter, WriteOptions},
};
use fnv::FnvBuildHasher;
use necsim_core::{
    landscape::{IndexedLocation, Location},
    lineage::GlobalLineageReference,
};
use necsim_core_bond::PositiveF64;

use crate::{LastEventState, SpeciesIdentity};

use super::LocationSpeciesFeatherReporter;

impl LocationSpeciesFeatherReporter {
    pub(super) fn store_individual_origin(
        &mut self,
        lineage: &GlobalLineageReference,
        origin: &Location,
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
    pub(super) fn output_to_dataframe(mut self) -> arrow2::error::Result<()> {
        let file = File::options()
            .write(true)
            .truncate(true)
            .open(&self.output)?;
        let writer = BufWriter::new(file);

        let expected_fields = vec![
            Field::new("x", DataType::UInt32, false),
            Field::new("y", DataType::UInt32, false),
            Field::new("species", DataType::FixedSizeBinary(24), false),
            Field::new("count", DataType::UInt64, false),
        ];

        let mut metadata = BTreeMap::new();
        metadata.insert(
            String::from("last-event"),
            LastEventState {
                last_parent_prior_time: self.last_parent_prior_time.clone(),
                last_speciation_event: self.last_speciation_event.clone(),
                last_dispersal_event: self.last_dispersal_event.clone(),
            }
            .into_string()
            .map_err(|()| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "failed to write metadata to species dataframe",
                )
            })?,
        );

        let mut writer = FileWriter::new(
            writer,
            Schema {
                fields: expected_fields,
                metadata,
            },
            None,
            WriteOptions { compression: None },
        );
        writer.start()?;

        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let mut species = Vec::new();
        let mut counts = Vec::new();

        let mut species_index: HashMap<(Location, SpeciesIdentity), usize, FnvBuildHasher> =
            HashMap::default();

        for (origin, identity, count) in std::mem::take(&mut self.speciated) {
            species_index.insert((origin.clone(), identity.clone()), counts.len());

            xs.push(origin.x());
            ys.push(origin.y());
            species.extend_from_slice(&*identity);
            counts.push(count);
        }

        let mut activities: HashMap<GlobalLineageReference, (PositiveF64, GlobalLineageReference)> =
            HashMap::default();

        // Lineage ancestor union-find with path compression
        let mut family = Vec::new();

        let mut unspeciated = Vec::new();

        for (lineage, origin) in std::mem::take(&mut self.origins) {
            // Find the ancestor that originated the species
            let mut ancestor = lineage.clone();
            while let Some(ancestor_parent) = self.parents.get(&ancestor) {
                family.push(ancestor.clone());
                ancestor = ancestor_parent.clone();
            }

            // Compress the ancestry paths for all visited lineages
            for child in family.drain(..) {
                self.parents.insert(child, ancestor.clone());
            }

            let count = self.counts.get(&lineage).copied().unwrap_or(1_u64);

            if let Some(identity) = self.species.get(&ancestor) {
                match species_index.entry((origin.clone(), identity.clone())) {
                    // Update the existing per-location-species record
                    Entry::Occupied(occupied) => counts[*occupied.get()] += count,
                    // Create a new per-location-species record
                    Entry::Vacant(vacant) => {
                        vacant.insert(counts.len());

                        xs.push(origin.x());
                        ys.push(origin.y());
                        species.extend_from_slice(&**identity);
                        counts.push(count);
                    },
                }

                continue;
            }

            if let Some(a) = self.activity.get(&lineage) {
                match activities.entry(ancestor.clone()) {
                    Entry::Occupied(mut occupied) => {
                        if *a > occupied.get().0 {
                            occupied.insert((*a, lineage.clone()));
                        }
                    },
                    Entry::Vacant(vacant) => {
                        vacant.insert((*a, lineage.clone()));
                    },
                }
            }

            unspeciated.push((lineage, origin, ancestor, count));
        }

        for (lineage, origin, ancestor, count) in unspeciated {
            // If no active frontier exists, every lineage must be considered
            //  to be part of the active frontier
            //  -> in this case the ancestor is a pseudo-anchor
            let (anchor_activity, anchor) = match activities.get(&ancestor) {
                Some((anchor_activity, anchor)) => (Some(anchor_activity), anchor.clone()),
                None => (None, ancestor.clone()),
            };

            // Lineages in the active frontier of the species-union,
            //  excluding the anchor, have unique location-species records
            if self.activity.get(&lineage) == anchor_activity && lineage != anchor {
                xs.push(origin.x());
                ys.push(origin.y());
                species.extend_from_slice(&*SpeciesIdentity::from_unspeciated(
                    lineage.clone(),
                    anchor.clone(),
                ));
                counts.push(count);

                continue;
            }

            // The unspeciated species-union has one fallback anchor
            let anchor_identity = SpeciesIdentity::from_unspeciated(anchor.clone(), anchor.clone());

            // No-longer activate lineages and the anchor may share
            //  location-species records with each other
            match species_index.entry((origin.clone(), anchor_identity.clone())) {
                // Update the existing per-location-species record
                Entry::Occupied(occupied) => counts[*occupied.get()] += count,
                // Create a new per-location-species record
                Entry::Vacant(vacant) => {
                    vacant.insert(counts.len());

                    xs.push(origin.x());
                    ys.push(origin.y());
                    species.extend_from_slice(&*anchor_identity);
                    counts.push(count);
                },
            }
        }

        let xs = PrimitiveArray::from_vec(xs);
        let ys = PrimitiveArray::from_vec(ys);
        let species = FixedSizeBinaryArray::try_new(
            DataType::FixedSizeBinary(24),
            Buffer::from(species),
            None,
        )?;
        let counts = PrimitiveArray::from_vec(counts);

        let chunk = Chunk::try_new(vec![
            xs.boxed(),
            ys.boxed(),
            species.boxed(),
            counts.boxed(),
        ])?;
        writer.write(&chunk, None)?;

        writer.finish()
    }
}
