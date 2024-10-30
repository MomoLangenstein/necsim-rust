use std::{
    convert::TryFrom,
    fmt,
    fs::{File, OpenOptions},
    io::{self, BufWriter, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use necsim_core::{
    event::{DispersalEvent, SpeciationEvent},
    impl_finalise, impl_report,
    reporter::Reporter,
};

#[derive(Deserialize)]
#[serde(try_from = "GlobalTurnoverReporterArgs")]
pub struct GlobalTurnoverReporter {
    last_speciation_event: Option<SpeciationEvent>,
    last_dispersal_event: Option<DispersalEvent>,

    output: PathBuf,
    writer: Option<BufWriter<File>>,
}

impl fmt::Debug for GlobalTurnoverReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(GlobalTurnoverReporter))
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for GlobalTurnoverReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        GlobalTurnoverReporterArgs {
            output: self.output.clone(),
        }
        .serialize(serializer)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalTurnoverReporterArgs {
    output: PathBuf,
}

impl TryFrom<GlobalTurnoverReporterArgs> for GlobalTurnoverReporter {
    type Error = io::Error;

    fn try_from(args: GlobalTurnoverReporterArgs) -> Result<Self, Self::Error> {
        // Preliminary argument parsing check if the output is a writable file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&args.output)?;
        std::mem::drop(file);

        Ok(Self {
            last_speciation_event: None,
            last_dispersal_event: None,

            output: args.output,
            writer: None,
        })
    }
}

impl GlobalTurnoverReporter {
    fn write_turnover(&mut self, turnover: f64) {
        if let Some(writer) = &mut self.writer {
            std::mem::drop(writeln!(writer, "{turnover}"));
        }
    }
}

impl Reporter for GlobalTurnoverReporter {
    impl_report!(speciation(&mut self, speciation: Used) {
        if Some(speciation) == self.last_speciation_event.as_ref() {
            return;
        }

        self.last_speciation_event = Some(speciation.clone());

        self.write_turnover(speciation.event_time.get() - speciation.prior_time.get());
    });

    impl_report!(dispersal(&mut self, dispersal: Used) {
        if Some(dispersal) == self.last_dispersal_event.as_ref() {
            return;
        }

        self.last_dispersal_event = Some(dispersal.clone());

        self.write_turnover(dispersal.event_time.get() - dispersal.prior_time.get());
    });

    impl_report!(progress(&mut self, _progress: Ignored) {});

    impl_finalise!((mut self) {
        if let Some(writer) = &mut self.writer {
            std::mem::drop(writer.flush());
        }
    });

    fn initialise(&mut self) -> Result<(), String> {
        if self.writer.is_some() {
            return Ok(());
        }

        let result = (|| -> io::Result<BufWriter<File>> {
            let file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&self.output)?;

            let mut writer = BufWriter::new(file);

            writeln!(writer, "turnover")?;

            Ok(writer)
        })();

        match result {
            Ok(writer) => {
                self.writer = Some(writer);

                Ok(())
            },
            Err(err) => Err(err.to_string()),
        }
    }
}
