use std::{
    convert::TryFrom,
    fmt,
    fs::{self, File, OpenOptions},
    path::PathBuf,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_state::DeserializeState;

use necsim_core_bond::NonNegativeF64;
use necsim_impls_std::lineage_file::saver::LineageFileSaver;
use necsim_partitioning_core::partition::PartitionSize;

#[derive(Debug, Serialize)]
pub struct Pause {
    pub before: NonNegativeF64,
    pub config: ResumeConfig,
    pub destiny: SampleDestiny,
    #[serde(default)]
    pub mode: PauseMode,
}

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Serialize, Deserialize)]
pub enum PauseMode {
    Resume,
    FixUp,
    Restart,
}

impl Default for PauseMode {
    fn default() -> Self {
        Self::Resume
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SampleDestiny {
    List,
    Bincode(LineageFileSaver),
}

#[derive(Deserialize)]
#[serde(try_from = "PathBuf")]
pub struct ResumeConfig {
    file: File,
    path: PathBuf,
    temp: bool,
}

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Serialize)]
#[serde(rename = "Pause")]
pub struct FuturePause {
    pub before: NonNegativeF64,
    pub mode: PauseMode,
}

impl<'de> DeserializeState<'de, PartitionSize> for Pause {
    fn deserialize_state<D: Deserializer<'de>>(
        partition_size: &mut PartitionSize,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let raw = PauseRaw::deserialize(deserializer)?;

        if !partition_size.is_monolithic() {
            return Err(serde::de::Error::custom(
                "Parallel pausing is not yet supported.",
            ));
        }

        if matches!(raw.mode, PauseMode::FixUp) && raw.before == NonNegativeF64::zero() {
            return Err(serde::de::Error::custom(
                "pause mode `FixUp` requires a positive non-zero pause time",
            ));
        }

        Ok(Pause {
            before: raw.before,
            config: raw.config,
            destiny: raw.destiny,
            mode: raw.mode,
        })
    }
}

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Pause")]
pub struct PauseRaw {
    pub before: NonNegativeF64,
    pub config: ResumeConfig,
    pub destiny: SampleDestiny,
    pub mode: PauseMode,
}

impl Drop for ResumeConfig {
    fn drop(&mut self) {
        if self.temp {
            std::mem::drop(fs::remove_file(self.path.clone()));
        }
    }
}

impl fmt::Debug for ResumeConfig {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.path.fmt(fmt)
    }
}

impl Serialize for ResumeConfig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.path.serialize(serializer)
    }
}

impl TryFrom<PathBuf> for ResumeConfig {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;

        Ok(Self {
            file,
            path,
            temp: true,
        })
    }
}

impl ResumeConfig {
    pub fn write(mut self, config: &str) -> anyhow::Result<()> {
        std::io::Write::write_fmt(&mut self.file, format_args!("{config}\n"))?;

        self.temp = false;

        Ok(())
    }
}
