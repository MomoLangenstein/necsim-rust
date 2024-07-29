use std::fmt;

use necsim_core::{event::SpeciationEvent, impl_finalise, impl_report, reporter::Reporter};

#[expect(clippy::module_name_repetitions)]
pub struct BiodiversityReporter {
    last_event: Option<SpeciationEvent>,

    biodiversity: usize,
}

impl fmt::Debug for BiodiversityReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(BiodiversityReporter))
            .field("biodiversity", &self.biodiversity)
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for BiodiversityReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}

impl<'de> serde::Deserialize<'de> for BiodiversityReporter {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::default())
    }
}

impl Reporter for BiodiversityReporter {
    impl_report!(speciation(&mut self, speciation: Used) {
        if Some(speciation) == self.last_event.as_ref() {
            return;
        }

        self.last_event = Some(speciation.clone());

        self.biodiversity += 1;
    });

    impl_report!(dispersal(&mut self, _dispersal: Ignored) {});

    impl_report!(progress(&mut self, _progress: Ignored) {});

    impl_finalise!((self) {
        if self.biodiversity > 0 {
            info!(
                "The simulation resulted in a biodiversity of {} unique species.",
                self.biodiversity
            );
        }
    });
}

impl Default for BiodiversityReporter {
    #[debug_ensures(ret.biodiversity == 0, "biodiversity initialised to 0")]
    fn default() -> Self {
        Self {
            last_event: None,
            biodiversity: 0,
        }
    }
}
