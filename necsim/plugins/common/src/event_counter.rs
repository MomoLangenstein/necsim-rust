use std::{fmt, fmt::Write};

use necsim_core::{
    event::{DispersalEvent, SpeciationEvent},
    impl_finalise, impl_report,
    lineage::LineageInteraction,
    reporter::Reporter,
};
use necsim_core_bond::NonNegativeF64;

#[expect(clippy::module_name_repetitions)]
#[derive(Default)]
pub struct EventCounterReporter {
    last_parent_prior_time: Option<NonNegativeF64>,
    last_speciation_event: Option<SpeciationEvent>,
    last_dispersal_event: Option<DispersalEvent>,

    raw_total: usize,
    speciation: usize,
    out_dispersal: usize,
    self_dispersal: usize,
    out_coalescence: usize,
    self_coalescence: usize,
    late_dispersal_coalescence: usize,
    late_coalescence: usize,
}

impl fmt::Debug for EventCounterReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(EventCounterReporter))
            .field("speciation", &self.speciation)
            .field("out_dispersal", &self.out_dispersal)
            .field("self_dispersal", &self.self_dispersal)
            .field("out_coalescence", &self.out_coalescence)
            .field("self_coalescence", &self.self_coalescence)
            .field(
                "late_dispersal",
                &(self.late_dispersal_coalescence - self.late_coalescence),
            )
            .field("late_coalescence", &self.late_coalescence)
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for EventCounterReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}

impl<'de> serde::Deserialize<'de> for EventCounterReporter {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::default())
    }
}

impl Reporter for EventCounterReporter {
    impl_report!(speciation(&mut self, speciation: Used) {
        self.raw_total += 1;

        if Some(speciation) == self.last_speciation_event.as_ref() {
            if Some(speciation.prior_time) != self.last_parent_prior_time {
                self.late_coalescence += 1;
            }
            self.last_parent_prior_time = Some(speciation.prior_time);

            return;
        }
        self.last_speciation_event = Some(speciation.clone());
        self.last_parent_prior_time = Some(speciation.prior_time);

        self.speciation += 1;
    });

    impl_report!(dispersal(&mut self, dispersal: Used) {
        self.raw_total += 1;

        if Some(dispersal) == self.last_dispersal_event.as_ref() {
            if Some(dispersal.prior_time) != self.last_parent_prior_time {
                self.late_coalescence += 1;
            }
            self.last_parent_prior_time = Some(dispersal.prior_time);

            return;
        }
        self.last_dispersal_event = Some(dispersal.clone());
        self.last_parent_prior_time = Some(dispersal.prior_time);

        let self_dispersal = dispersal.origin == dispersal.target;
        let coalescence = if dispersal.interaction == LineageInteraction::Maybe {
            self.late_dispersal_coalescence += 1;
            return
        } else { dispersal.interaction.is_coalescence() };

        match (self_dispersal, coalescence) {
            (true, true) => {
                self.self_coalescence += 1;
            },
            (true, false) => {
                self.self_dispersal += 1;
            },
            (false, true) => {
                self.out_coalescence += 1;
            },
            (false, false) => {
                self.out_dispersal += 1;
            },
        }
    });

    impl_report!(progress(&mut self, _progress: Ignored) {});

    impl_finalise!((self) {
        if self.last_speciation_event.is_none() && self.last_dispersal_event.is_none() {
            return;
        }

        let mut event_summary = String::new();

        let _ = writeln!(&mut event_summary, "Event Summary:");

        let _ = writeln!(
            &mut event_summary,
            " - Total #individuals:\n   {}",
            self.speciation + self.self_coalescence + self.out_coalescence + self.late_coalescence
        );
        let _ = writeln!(
            &mut event_summary,
            " - Total #events:\
            \n   - raw:\n     {}\
            \n   - deduplicated:\n     {}",
            self.raw_total,
            self.speciation
                + self.self_coalescence
                + self.out_coalescence
                + self.self_dispersal
                + self.out_dispersal
                + self.late_dispersal_coalescence
        );
        let _ = writeln!(
            &mut event_summary,
            " - Speciation:\
            \n    {}",
            self.speciation
        );
        let _ = writeln!(
            &mut event_summary,
            " - Dispersal:\
            \n   - same location, no coalescence:\n     {}\
            \n   - same location, with coalescence:\n     {}\
            \n   - new location, no coalescence:\n     {}\
            \n   - new location, with coalescence:\n     {}\
            \n   - detected late, no coalescence:\n     {}\
            \n   - detected late, with coalescence:\n     {}",
            self.self_dispersal,
            self.self_coalescence,
            self.out_dispersal,
            self.out_coalescence,
            self.late_dispersal_coalescence - self.late_coalescence,
            self.late_coalescence
        );

        log::info!("{}", event_summary);
    });
}
