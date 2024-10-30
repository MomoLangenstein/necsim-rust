#![deny(clippy::pedantic)]

#[macro_use]
extern crate log;

use std::{collections::HashSet, fmt, num::NonZeroU64};

use fnv::FnvBuildHasher;
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use necsim_core::{event::SpeciationEvent, impl_finalise, impl_report, reporter::Reporter};

necsim_plugins_core::export_plugin!(Metacommunity => MetacommunityMigrationReporter);

#[derive(Deserialize)]
#[serde(from = "MetacommunityMigrationReporterArgs")]
pub struct MetacommunityMigrationReporter {
    last_event: Option<SpeciationEvent>,

    metacommunity: Metacommunity,
    seed: u64,

    migrations: usize,
}

impl fmt::Debug for MetacommunityMigrationReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(MetacommunityMigrationReporter))
            .field("metacommunity", &self.metacommunity)
            .field("seed", &self.seed)
            .field("migrations", &self.migrations)
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for MetacommunityMigrationReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        MetacommunityMigrationReporterArgs {
            metacommunity: self.metacommunity,
            seed: self.seed,
        }
        .serialize(serializer)
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
enum Metacommunity {
    Infinite,
    Finite(NonZeroU64),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetacommunityMigrationReporterArgs {
    metacommunity: Metacommunity,
    seed: u64,
}

impl From<MetacommunityMigrationReporterArgs> for MetacommunityMigrationReporter {
    fn from(args: MetacommunityMigrationReporterArgs) -> Self {
        Self {
            last_event: None,

            metacommunity: args.metacommunity,
            seed: args.seed,

            migrations: 0_usize,
        }
    }
}

impl Reporter for MetacommunityMigrationReporter {
    impl_report!(speciation(&mut self, speciation: Used) {
        if Some(speciation) == self.last_event.as_ref() {
            return;
        }

        self.last_event = Some(speciation.clone());

        self.migrations += 1;
    });

    impl_report!(dispersal(&mut self, _dispersal: Ignored) {});

    impl_report!(progress(&mut self, _progress: Ignored) {});

    impl_finalise!((self) {
        if self.migrations == 0 {
            return
        }

        let metacommunity_size = match self.metacommunity {
            Metacommunity::Infinite => {
                return info!(
                    "There were {} migrations to an infinite metacommunity during the simulation.",
                    self.migrations
                )
            },
            Metacommunity::Finite(metacommunity_size) => metacommunity_size,
        };

        let mut rng = StdRng::seed_from_u64(self.seed);

        let mut unique_migration_targets =
            HashSet::with_capacity_and_hasher(self.migrations, FnvBuildHasher::default());

        for _ in 0..self.migrations {
            unique_migration_targets.insert(rng.gen_range(0..metacommunity_size.get()));
        }

        info!(
            "There were {} migrations to {} ancestors on a finite metacommunity of \
            size {} during the simulation.",
            self.migrations, unique_migration_targets.len(), metacommunity_size,
        );
    });
}
