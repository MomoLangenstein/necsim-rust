use std::{fmt, time::Instant};

use necsim_core::{impl_finalise, impl_report, reporter::Reporter};

#[expect(clippy::module_name_repetitions)]
pub struct ExecutionTimeReporter {
    init_time: Instant,
    start_time: Option<Instant>,
    end_time: Option<Instant>,
}

impl fmt::Debug for ExecutionTimeReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(ExecutionTimeReporter))
            .field(
                "start_time",
                &self
                    .start_time
                    .as_ref()
                    .map(|time| time.duration_since(self.init_time)),
            )
            .field(
                "end_time",
                &self
                    .end_time
                    .as_ref()
                    .map(|time| time.duration_since(self.init_time)),
            )
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for ExecutionTimeReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}

impl<'de> serde::Deserialize<'de> for ExecutionTimeReporter {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::default())
    }
}

impl Reporter for ExecutionTimeReporter {
    impl_report!(speciation(&mut self, _speciation: Ignored) {});

    impl_report!(dispersal(&mut self, _dispersal: Ignored) {});

    impl_report!(
        #[debug_ensures(self.start_time.is_some(), "start_time is set after first call")]
        progress(&mut self, remaining: Used) {
            if self.start_time.is_none() {
                self.start_time = Some(Instant::now());
            }

            if *remaining == 0 {
                self.end_time = Some(Instant::now());
            }
        }
    );

    impl_finalise!((mut self) {
        match (self.start_time, self.end_time) {
            (Some(start_time), Some(end_time)) => info!(
                "The simulation took:\n - initialisation: {:?}\n - execution: {:?}\n - \
                cleanup: {:?}",
                (start_time - self.init_time),
                (end_time - start_time),
                end_time.elapsed()
            ),
            (Some(start_time), None) => info!(
                "The simulation took:\n - initialisation: {:?}\n - execution: {:?}",
                (start_time - self.init_time),
                start_time.elapsed(),
            ),
            _ => (),
        };
    });

    fn initialise(&mut self) -> Result<(), String> {
        self.init_time = Instant::now();
        self.start_time = None;
        self.end_time = None;

        Ok(())
    }
}

impl Default for ExecutionTimeReporter {
    #[debug_ensures(ret.start_time.is_none(), "initialises start_time to None")]
    #[debug_ensures(ret.end_time.is_none(), "initialises end_time to None")]
    fn default() -> Self {
        Self {
            init_time: Instant::now(),
            start_time: None,
            end_time: None,
        }
    }
}
