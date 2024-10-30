use std::{
    collections::BinaryHeap, convert::TryFrom, iter::FromIterator, num::NonZeroUsize, path::Path,
};

use serde::{Deserialize, Serialize, Serializer};

use necsim_core::event::PackedEvent;

mod globbed;
pub mod segment;
mod sorted_segments;

use globbed::GlobbedSortedSegments;
use segment::SortedSegment;
use sorted_segments::SortedSortedSegments;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Deserialize)]
#[serde(try_from = "EventLogReplayRaw")]
pub struct EventLogReplay {
    frontier: BinaryHeap<SortedSortedSegments>,

    with_speciation: bool,
    with_dispersal: bool,
}

impl Serialize for EventLogReplay {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct EventLog<'r> {
            segments: Vec<&'r Path>,
            capacity: NonZeroUsize,
        }

        let mut segments = Vec::new();
        let mut capacity = NonZeroUsize::new(1).unwrap();

        for segs in &self.frontier {
            for seg in segs.segments() {
                segments.push(seg.path());

                capacity = capacity.max(seg.capacity());
            }
        }

        segments.sort_unstable();

        EventLog { segments, capacity }.serialize(serializer)
    }
}

impl TryFrom<EventLogReplayRaw> for EventLogReplay {
    type Error = anyhow::Error;

    fn try_from(raw: EventLogReplayRaw) -> Result<Self, Self::Error> {
        let capacity = raw.capacity;

        raw.segments
            .into_iter()
            .flatten()
            .map(|mut segment| {
                segment.set_capacity(capacity);
                segment
            })
            .collect()
    }
}

impl EventLogReplay {
    #[must_use]
    pub fn length(&self) -> usize {
        self.frontier.iter().map(SortedSortedSegments::length).sum()
    }

    #[must_use]
    pub fn with_speciation(&self) -> bool {
        self.with_speciation
    }

    #[must_use]
    pub fn with_dispersal(&self) -> bool {
        self.with_dispersal
    }
}

impl FromIterator<SortedSegment> for anyhow::Result<EventLogReplay> {
    fn from_iter<T: IntoIterator<Item = SortedSegment>>(iter: T) -> Self {
        let mut segments: Vec<SortedSegment> = iter.into_iter().collect();

        if segments.is_empty() {
            anyhow::bail!("The EventLogReplay requires at least one event log segment.")
        }

        let mut with_speciation = None;
        let mut with_dispersal = None;

        for segment in &segments {
            if let Some(with_speciation) = with_speciation {
                anyhow::ensure!(
                    with_speciation == segment.header().with_speciation(),
                    "There is a mismatch in reporting speciation events between some segments."
                );
            } else {
                with_speciation = Some(segment.header().with_speciation());
            }

            if let Some(with_dispersal) = with_dispersal {
                anyhow::ensure!(
                    with_dispersal == segment.header().with_dispersal(),
                    "There is a mismatch in reporting dispersal events between some segments."
                );
            } else {
                with_dispersal = Some(segment.header().with_dispersal());
            }
        }

        let mut grouped_segments: Vec<Vec<SortedSegment>> = Vec::new();
        let mut current_group: Vec<SortedSegment> = Vec::new();

        let mut max_time: f64 = f64::NEG_INFINITY;

        while !segments.is_empty() {
            let mut min_time: f64 = f64::INFINITY;
            let mut min_index: Option<usize> = None;

            for (i, seg) in segments.iter().enumerate() {
                if seg.header().min_time() > max_time && seg.header().min_time() < min_time {
                    min_time = seg.header().min_time().get();
                    min_index = Some(i);
                }
            }

            let Some(min_index) = min_index else {
                if !current_group.is_empty() {
                    grouped_segments.push(current_group);
                    current_group = Vec::new();
                }

                max_time = f64::NEG_INFINITY;

                continue;
            };

            let min_segement = segments.swap_remove(min_index);

            max_time = min_segement.header().max_time().get();

            current_group.push(min_segement);
        }

        if !current_group.is_empty() {
            grouped_segments.push(current_group);
        }

        let mut frontier = BinaryHeap::with_capacity(grouped_segments.len());

        for group in grouped_segments {
            frontier.push(SortedSortedSegments::new(group));
        }

        Ok(EventLogReplay {
            frontier,
            with_speciation: with_speciation.unwrap(),
            with_dispersal: with_dispersal.unwrap(),
        })
    }
}

impl Iterator for EventLogReplay {
    type Item = PackedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let mut next_segment = self.frontier.pop()?;

        let next_event = next_segment.next();

        self.frontier.push(next_segment);

        next_event
    }
}

#[derive(Deserialize)]
#[serde(rename = "EventLog")]
#[serde(deny_unknown_fields)]
struct EventLogReplayRaw {
    segments: Vec<GlobbedSortedSegments>,
    #[serde(default = "default_event_log_replay_segment_capacity")]
    capacity: NonZeroUsize,
}

fn default_event_log_replay_segment_capacity() -> NonZeroUsize {
    NonZeroUsize::new(100_000_usize).unwrap()
}
