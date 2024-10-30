use std::{
    convert::TryFrom,
    fmt,
    fs::{File, OpenOptions},
    io::{self, BufWriter, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use necsim_core::{event::DispersalEvent, impl_finalise, impl_report, reporter::Reporter};

#[derive(Deserialize)]
#[serde(try_from = "GlobalCoverageReporterArgs")]
pub struct GlobalCoverageReporter {
    last_dispersal_event: Option<DispersalEvent>,

    output: PathBuf,
    writer: Option<BufWriter<File>>,
}

impl fmt::Debug for GlobalCoverageReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(GlobalCoverageReporter))
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for GlobalCoverageReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        GlobalCoverageReporterArgs {
            output: self.output.clone(),
        }
        .serialize(serializer)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalCoverageReporterArgs {
    output: PathBuf,
}

impl TryFrom<GlobalCoverageReporterArgs> for GlobalCoverageReporter {
    type Error = io::Error;

    fn try_from(args: GlobalCoverageReporterArgs) -> Result<Self, Self::Error> {
        // Preliminary argument parsing check if the output is a writable file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&args.output)?;
        std::mem::drop(file);

        Ok(Self {
            last_dispersal_event: None,

            output: args.output,
            writer: None,
        })
    }
}

impl Reporter for GlobalCoverageReporter {
    impl_report!(speciation(&mut self, _speciation: Ignored) {});

    impl_report!(dispersal(&mut self, dispersal: Used) {
        if Some(dispersal) == self.last_dispersal_event.as_ref() {
            return;
        }

        self.last_dispersal_event = Some(dispersal.clone());

        if let Some(writer) = &mut self.writer {
            std::mem::drop(writeln!(
                writer, "{},{},{},{},{},{}",
                dispersal.origin.location().x(), dispersal.origin.location().y(),
                dispersal.origin.index(),
                dispersal.target.location().x(), dispersal.target.location().y(),
                dispersal.target.index(),
            ));
        }
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

            writeln!(writer, "x-from,y-from,index-from,x-to,y-to,index-to")?;

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
