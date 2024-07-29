use std::{
    convert::TryFrom,
    fmt,
    fs::{File, OpenOptions},
    io::{self, BufWriter, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use necsim_core::{event::SpeciationEvent, impl_finalise, impl_report, reporter::Reporter};

#[derive(Deserialize)]
#[serde(try_from = "GlobalSpeciationReporterArgs")]
pub struct GlobalSpeciationReporter {
    last_speciation_event: Option<SpeciationEvent>,

    output: PathBuf,
    writer: Option<BufWriter<File>>,
}

impl fmt::Debug for GlobalSpeciationReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(GlobalSpeciationReporter))
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for GlobalSpeciationReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        GlobalSpeciationReporterArgs {
            output: self.output.clone(),
        }
        .serialize(serializer)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalSpeciationReporterArgs {
    output: PathBuf,
}

impl TryFrom<GlobalSpeciationReporterArgs> for GlobalSpeciationReporter {
    type Error = io::Error;

    fn try_from(args: GlobalSpeciationReporterArgs) -> Result<Self, Self::Error> {
        // Preliminary argument parsing check if the output is a writable file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&args.output)?;
        std::mem::drop(file);

        Ok(Self {
            last_speciation_event: None,

            output: args.output,
            writer: None,
        })
    }
}

impl Reporter for GlobalSpeciationReporter {
    impl_report!(speciation(&mut self, speciation: Used) {
        if Some(speciation) == self.last_speciation_event.as_ref() {
            return;
        }

        self.last_speciation_event = Some(speciation.clone());

        if let Some(writer) = &mut self.writer {
            std::mem::drop(writeln!(writer, "{}", speciation.event_time.get()));
        }
    });

    impl_report!(dispersal(&mut self, _dispersal: Ignored) {});

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

            writeln!(writer, "speciation")?;

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
