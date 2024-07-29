use std::{
    convert::TryFrom,
    fs::OpenOptions,
    io::BufReader,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize, Serializer};

use necsim_core::lineage::Lineage;

#[derive(Debug, Deserialize, Clone)]
#[serde(try_from = "LineageFileLoaderRaw")]
#[expect(clippy::module_name_repetitions)]
pub struct LineageFileLoader {
    lineages: Vec<Lineage>,
    path: PathBuf,
}

impl Serialize for LineageFileLoader {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        LineageFileLoaderRaw {
            file: self.path.clone(),
        }
        .serialize(serializer)
    }
}

impl LineageFileLoader {
    /// # Errors
    ///
    /// Fails if the `path` cannot be read as a list of lineages
    pub fn try_new(path: &Path) -> anyhow::Result<Self> {
        let file = OpenOptions::new().read(true).write(false).open(path)?;

        let mut deserializer = rmp_serde::Deserializer::new(BufReader::new(file));
        // bincode::Deserializer::with_reader(BufReader::new(file), bincode::options());

        let lineages = <Vec<Lineage>>::deserialize(&mut deserializer)?;

        Ok(Self {
            lineages,
            path: path.to_owned(),
        })
    }

    #[must_use]
    pub fn into_lineages(self) -> Vec<Lineage> {
        self.lineages
    }

    #[must_use]
    pub fn get_lineages(&self) -> &[Lineage] {
        &self.lineages
    }
}

impl TryFrom<LineageFileLoaderRaw> for LineageFileLoader {
    type Error = anyhow::Error;

    fn try_from(raw: LineageFileLoaderRaw) -> Result<Self, Self::Error> {
        Self::try_new(&raw.file)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "LineageFileLoader")]
#[serde(deny_unknown_fields)]
struct LineageFileLoaderRaw {
    file: PathBuf,
}
