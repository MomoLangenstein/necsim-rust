use std::{
    convert::TryFrom,
    fmt,
    fs::{self, File, OpenOptions},
    io::BufWriter,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize, Serializer};

use necsim_core::lineage::Lineage;

#[derive(Deserialize)]
#[serde(try_from = "LineageFileSaverRaw")]
#[expect(clippy::module_name_repetitions)]
pub struct LineageFileSaver {
    file: File,
    path: PathBuf,
    temp: bool,
}

impl Drop for LineageFileSaver {
    fn drop(&mut self) {
        if self.temp {
            std::mem::drop(fs::remove_file(&self.path));
        }
    }
}

impl fmt::Debug for LineageFileSaver {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(LineageFileSaver))
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl Serialize for LineageFileSaver {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        LineageFileSaverRaw {
            file: self.path.clone(),
        }
        .serialize(serializer)
    }
}

impl LineageFileSaver {
    /// # Errors
    ///
    /// Fails if a new lineage file cannot be created at `path`
    pub fn try_new(path: &Path) -> anyhow::Result<Self> {
        let file = OpenOptions::new().create_new(true).write(true).open(path)?;

        Ok(Self {
            file,
            path: path.to_owned(),
            temp: true,
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// # Errors
    ///
    /// Fails if a the lineages could not be written to the file at `path`
    pub fn write<'a, I: Iterator<Item = &'a Lineage>>(mut self, lineages: I) -> anyhow::Result<()> {
        let mut serializer = rmp_serde::Serializer::new(BufWriter::new(&mut self.file));
        // bincode::Serializer::new(BufWriter::new(&mut self.file), bincode::options());

        serializer.collect_seq(lineages)?;

        self.temp = false;

        Ok(())
    }
}

impl TryFrom<LineageFileSaverRaw> for LineageFileSaver {
    type Error = anyhow::Error;

    fn try_from(raw: LineageFileSaverRaw) -> Result<Self, Self::Error> {
        Self::try_new(&raw.file)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "LineageFileSaver")]
#[serde(deny_unknown_fields)]
struct LineageFileSaverRaw {
    file: PathBuf,
}
