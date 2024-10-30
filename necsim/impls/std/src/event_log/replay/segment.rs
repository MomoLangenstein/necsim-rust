use std::{
    cmp::{Ord, Ordering},
    collections::VecDeque,
    fmt,
    fs::{File, OpenOptions},
    io::BufReader,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use anyhow::Result;

use necsim_core::event::PackedEvent;

use crate::event_log::EventLogHeader;

#[expect(clippy::module_name_repetitions)]
pub struct SortedSegment {
    path: PathBuf,
    header: EventLogHeader,
    reader: BufReader<File>,
    buffer: VecDeque<PackedEvent>,
    capacity: NonZeroUsize,
}

impl fmt::Debug for SortedSegment {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(SortedSegment))
            .field("path", &self.path)
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}

impl SortedSegment {
    /// # Errors
    ///
    /// Fails if the `path` cannot be read as an event log segment
    pub fn try_new(path: &Path, capacity: NonZeroUsize) -> Result<Self> {
        let file = OpenOptions::new().read(true).write(false).open(path)?;

        let mut buf_reader = BufReader::new(file);

        let header: EventLogHeader = bincode::deserialize_from(&mut buf_reader)?;

        let mut buffer = VecDeque::with_capacity(header.length.min(capacity.get()));

        if let Ok(event) = bincode::deserialize_from(&mut buf_reader) {
            buffer.push_back(event);
        }

        Ok(Self {
            path: path.to_owned(),
            header,
            reader: buf_reader,
            buffer,
            capacity,
        })
    }

    pub fn set_capacity(&mut self, capacity: NonZeroUsize) {
        if let Some(additional) = capacity.get().checked_sub(self.capacity.get()) {
            self.buffer.reserve(additional);
        }

        self.capacity = capacity;
    }

    #[must_use]
    pub fn header(&self) -> &EventLogHeader {
        &self.header
    }

    #[must_use]
    pub fn length(&self) -> usize {
        self.header.length()
    }

    #[must_use]
    pub fn capacity(&self) -> NonZeroUsize {
        self.capacity
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Iterator for SortedSegment {
    type Item = PackedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let next_event = self.buffer.pop_front();

        if next_event.is_some() && self.buffer.is_empty() {
            for _ in 0..self.capacity.get() {
                if let Ok(event) = bincode::deserialize_from(&mut self.reader) {
                    self.buffer.push_back(event);
                } else {
                    break;
                }
            }
        }

        next_event
    }
}

impl Ord for SortedSegment {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.buffer.front(), other.buffer.front()) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Less,
            (_, None) => Ordering::Greater,
            (Some(this_event), Some(other_event)) => other_event.cmp(this_event),
        }
    }
}

impl PartialOrd for SortedSegment {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for SortedSegment {
    fn eq(&self, other: &Self) -> bool {
        self.buffer.front() == other.buffer.front()
    }
}

impl Eq for SortedSegment {}
