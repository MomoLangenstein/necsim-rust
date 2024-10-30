// Based on https://docs.rs/extsort/0.4.2/src/extsort/sorter.rs.html

// Copyright 2018 Andre-Philippe Paquet
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    borrow::Cow,
    convert::TryFrom,
    fmt,
    fs::{self, OpenOptions},
    io::BufWriter,
    mem::ManuallyDrop,
    num::NonZeroUsize,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Error, Result};
use serde::{Deserialize, Serialize, Serializer};

use necsim_core::event::{DispersalEvent, PackedEvent, SpeciationEvent};

use super::EventLogHeader;

#[expect(clippy::module_name_repetitions)]
pub struct EventLogRecorder {
    segment_capacity: NonZeroUsize,
    directory: PathBuf,
    segment_index: usize,
    buffer: Vec<PackedEvent>,

    record_speciation: bool,
    record_dispersal: bool,
}

impl Drop for EventLogRecorder {
    fn drop(&mut self) {
        if !self.buffer.is_empty() {
            std::mem::drop(self.sort_and_write_segment());
        }

        // Try to remove the directory if it is empty
        std::mem::drop(fs::remove_dir(&self.directory));
    }
}

impl EventLogRecorder {
    /// # Errors
    ///
    /// Fails to construct iff `directory` is not a writable directory.
    pub fn try_new(directory: PathBuf, segment_capacity: NonZeroUsize) -> Result<Self> {
        if let Some(parent) = directory.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to ensure that the parent path for {directory:?} exists")
            })?;
        }

        Self {
            segment_capacity,
            directory,
            segment_index: 0_usize,
            buffer: Vec::with_capacity(segment_capacity.get()),

            record_speciation: false,
            record_dispersal: false,
        }
        .create_valid_directory()
    }

    /// # Errors
    ///
    /// Fails to construct iff
    /// - `child` is not a valid single-component path
    /// - newly creating a writable child directory fails
    pub fn new_child_log(&self, child: &str) -> Result<Self> {
        Self::check_valid_component(child)?;

        Self {
            segment_capacity: self.segment_capacity,
            directory: self.directory.join(child),
            segment_index: 0,
            buffer: Vec::with_capacity(self.segment_capacity.get()),
            record_speciation: self.record_speciation,
            record_dispersal: self.record_dispersal,
        }
        .create_valid_directory()
    }

    fn create_valid_directory(mut self) -> Result<Self> {
        // TODO: MPI cannot newly co-create all entries
        fs::create_dir(&self.directory).with_context(|| {
            format!(
                "failed to newly create the directory {:?}\n\nIf you are starting a new \
                 simulation, clean out the existing log.\nIf you are pausing or resuming a \
                 simulation, try appending a simulation-slice-specific postfix to your log path, \
                 and keep all these log-slices in the same parent directory for easy analysis.",
                self.directory
            )
        })?;

        self.directory = self.directory.canonicalize()?;

        let metadata = fs::metadata(&self.directory)?;

        if !metadata.is_dir() {
            return Err(anyhow::anyhow!("{:?} is not a directory.", self.directory));
        }

        if metadata.permissions().readonly() {
            return Err(anyhow::anyhow!(
                "{:?} is a read-only directory.",
                self.directory
            ));
        }

        Ok(self)
    }

    fn check_valid_component(component: &str) -> Result<()> {
        let mut child_components = Path::new(component).components();

        anyhow::ensure!(
            matches!(child_components.next(), Some(Component::Normal(first)) if first == component),
            "{component:?} is not a regular path component"
        );
        anyhow::ensure!(
            child_components.next().is_none(),
            "{component:?} must be a singular path component"
        );

        Ok(())
    }

    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn set_event_filter(&mut self, record_speciation: bool, record_dispersal: bool) {
        self.record_speciation = record_speciation;
        self.record_dispersal = record_dispersal;
    }

    pub fn record_speciation(&mut self, event: &SpeciationEvent) {
        self.record_speciation = true;

        self.buffer.push(event.clone().into());

        if self.buffer.len() >= self.segment_capacity.get() {
            std::mem::drop(self.sort_and_write_segment());
        }
    }

    pub fn record_dispersal(&mut self, event: &DispersalEvent) {
        self.record_dispersal = true;

        self.buffer.push(event.clone().into());

        if self.buffer.len() >= self.segment_capacity.get() {
            std::mem::drop(self.sort_and_write_segment());
        }
    }

    fn sort_and_write_segment(&mut self) -> Result<()> {
        self.buffer.sort_unstable();

        let segment_path = self.directory.join(format!("{}", self.segment_index));
        self.segment_index += 1;

        let segment_file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(segment_path)?;
        let mut buf_writer = BufWriter::new(segment_file);

        bincode::serialize_into(
            &mut buf_writer,
            &EventLogHeader::new(
                self.buffer[0].event_time(),
                self.buffer[self.buffer.len() - 1].event_time(),
                self.buffer.len(),
                self.record_speciation,
                self.record_dispersal,
            ),
        )?;

        for event in self.buffer.drain(0..) {
            bincode::serialize_into(&mut buf_writer, &event)?;
        }

        buf_writer.into_inner()?;

        Ok(())
    }
}

impl fmt::Debug for EventLogRecorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct EventBufferLen(usize);

        impl fmt::Debug for EventBufferLen {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "Vec<Event; {}>", self.0)
            }
        }

        f.debug_struct(stringify!(EventLogRecorder))
            .field("segment_capacity", &self.segment_capacity)
            .field("directory", &self.directory)
            .field("segment_index", &self.segment_index)
            .field("buffer", &EventBufferLen(self.buffer.len()))
            .finish_non_exhaustive()
    }
}

#[expect(clippy::unsafe_derive_deserialize)]
#[derive(Debug, Deserialize)]
#[serde(try_from = "EventLogRecorderRaw")]
pub struct EventLogConfig {
    directory: PathBuf,
    #[serde(default = "default_event_log_recorder_segment_capacity")]
    capacity: NonZeroUsize,
}

impl<'a> TryFrom<EventLogRecorderRaw<'a>> for EventLogConfig {
    type Error = Error;

    fn try_from(raw: EventLogRecorderRaw) -> Result<Self, Self::Error> {
        Self::try_new(raw.directory.into_owned(), raw.capacity)
    }
}

impl Serialize for EventLogConfig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        EventLogRecorderRaw {
            directory: Cow::Borrowed(&self.directory),
            capacity: self.capacity,
        }
        .serialize(serializer)
    }
}

impl EventLogConfig {
    /// # Errors
    ///
    /// Fails to construct iff the parent of `directory` cannot be created or
    /// is not a writable directory.
    pub fn try_new(directory: PathBuf, capacity: NonZeroUsize) -> Result<Self> {
        Self {
            directory,
            capacity,
        }
        .create_parent_directory()
    }

    /// # Errors
    ///
    /// Fails to construct iff
    /// - `child` is not a valid single-component path
    /// - newly creating a writable child directory fails
    pub fn new_child_log(&self, child: &str) -> Result<Self> {
        EventLogRecorder::check_valid_component(child)?;

        Self {
            directory: self.directory.join(child),
            capacity: self.capacity,
        }
        .create_parent_directory()
    }

    fn create_parent_directory(mut self) -> Result<Self> {
        let Some(name) = self.directory.file_name() else {
            anyhow::bail!(
                "{:?} does not terminate in a directory name",
                self.directory
            );
        };

        let Some(parent) = self.directory.parent() else {
            return Ok(self);
        };
        let parent = if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        };

        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to ensure that the parent path for {:?} exists",
                self.directory
            )
        })?;

        let mut directory = parent.canonicalize()?;
        directory.push(name);
        self.directory = directory;

        let Some(parent) = self.directory.parent() else {
            return Ok(self);
        };

        let metadata = fs::metadata(parent)?;

        if !metadata.is_dir() {
            return Err(anyhow::anyhow!(
                "the parent path of {:?} is not a directory.",
                self.directory
            ));
        }

        if metadata.permissions().readonly() {
            return Err(anyhow::anyhow!(
                "the parent path of {:?} is a read-only directory.",
                self.directory
            ));
        }

        Ok(self)
    }

    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// # Errors
    ///
    /// Fails to construct iff `self.directory()` is not a writable directory.
    pub fn create(self) -> Result<EventLogRecorder> {
        let this = ManuallyDrop::new(self);
        // Safety: self will not be dropped and self.directory is only read once
        let directory = unsafe { std::ptr::read(&this.directory) };
        EventLogRecorder::try_new(directory, this.capacity)
    }
}

impl Drop for EventLogConfig {
    fn drop(&mut self) {
        // Try to remove the directory parent if it is empty
        if let Some(parent) = self.directory.parent() {
            std::mem::drop(fs::remove_dir(parent));
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename = "EventLog")]
#[serde(deny_unknown_fields)]
struct EventLogRecorderRaw<'a> {
    #[serde(borrow)]
    directory: Cow<'a, Path>,
    #[serde(default = "default_event_log_recorder_segment_capacity")]
    capacity: NonZeroUsize,
}

fn default_event_log_recorder_segment_capacity() -> NonZeroUsize {
    NonZeroUsize::new(1_000_000_usize).unwrap()
}
