#![deny(clippy::pedantic)]

use std::{
    convert::TryFrom,
    fmt,
    fs::{File, OpenOptions},
    io::{self, BufWriter, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use necsim_core::{
    impl_finalise, impl_report, landscape::IndexedLocation, lineage::GlobalLineageReference,
    reporter::Reporter,
};

necsim_plugins_core::export_plugin!(Csv => CsvReporter);

#[derive(Deserialize)]
#[serde(try_from = "CsvReporterArgs")]
pub struct CsvReporter {
    output: PathBuf,
    writer: Option<BufWriter<File>>,
}

impl fmt::Debug for CsvReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(CsvReporter))
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for CsvReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        CsvReporterArgs {
            output: self.output.clone(),
        }
        .serialize(serializer)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CsvReporterArgs {
    output: PathBuf,
}

impl TryFrom<CsvReporterArgs> for CsvReporter {
    type Error = io::Error;

    fn try_from(args: CsvReporterArgs) -> Result<Self, Self::Error> {
        // Preliminary argument parsing check if the output is a writable file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&args.output)?;
        std::mem::drop(file);

        Ok(Self {
            output: args.output,
            writer: None,
        })
    }
}

impl Reporter for CsvReporter {
    impl_report!(speciation(&mut self, speciation: Used) {
        self.write_event(
            &speciation.global_lineage_reference,
            speciation.event_time.get(), &speciation.origin, 's'
        );
    });

    impl_report!(dispersal(&mut self, dispersal: Used) {
        self.write_event(
            &dispersal.global_lineage_reference,
            dispersal.event_time.get(), &dispersal.origin, 'd'
        );
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

            writeln!(writer, "reference,time,x,y,index,type")?;

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

impl CsvReporter {
    fn write_event(
        &mut self,
        reference: &GlobalLineageReference,
        time: f64,
        origin: &IndexedLocation,
        r#type: char,
    ) {
        if let Some(writer) = &mut self.writer {
            std::mem::drop(writeln!(
                writer,
                "{},{},{},{},{},{}",
                reference,
                time,
                origin.location().x(),
                origin.location().y(),
                origin.index(),
                r#type,
            ));
        }
    }
}
