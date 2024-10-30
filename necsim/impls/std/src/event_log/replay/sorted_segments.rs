use std::{
    cmp::{Ord, Ordering},
    fmt,
};

use necsim_core::event::PackedEvent;

use super::segment::SortedSegment;

#[expect(clippy::module_name_repetitions)]
pub struct SortedSortedSegments {
    segments: Vec<SortedSegment>,
    next: Option<PackedEvent>,
}

impl fmt::Debug for SortedSortedSegments {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct SortedSegmentFmt(usize);

        impl fmt::Debug for SortedSegmentFmt {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "Vec<SortedSegment; {}>", self.0)
            }
        }

        let mut debug = fmt.debug_struct(stringify!(SortedSortedSegments));
        debug.field("segments", &SortedSegmentFmt(self.segments.len()));

        if let (Some(last), Some(first)) = (self.segments.first(), self.segments.last()) {
            debug.field("min_time", &first.header().min_time());
            debug.field("max_time", &last.header().max_time());
        }

        debug
            .field("length", &self.length())
            .finish_non_exhaustive()
    }
}

impl SortedSortedSegments {
    pub fn new(mut segments: Vec<SortedSegment>) -> Self {
        segments.reverse();

        let mut this = Self {
            segments,
            next: None,
        };

        this.next();

        this
    }

    pub fn length(&self) -> usize {
        self.segments.iter().map(SortedSegment::length).sum()
    }

    pub fn segments(&self) -> &[SortedSegment] {
        &self.segments
    }
}

impl Iterator for SortedSortedSegments {
    type Item = PackedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let next = std::mem::replace(
            &mut self.next,
            loop {
                let Some(next_segment) = self.segments.last_mut() else {
                    break None;
                };

                if let Some(next_event) = next_segment.next() {
                    break Some(next_event);
                }

                self.segments.pop();
            },
        );

        next
    }
}

impl Ord for SortedSortedSegments {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.next, &other.next) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Less,
            (_, None) => Ordering::Greater,
            (Some(this_event), Some(other_event)) => other_event.cmp(this_event),
        }
    }
}

impl PartialOrd for SortedSortedSegments {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for SortedSortedSegments {
    fn eq(&self, other: &Self) -> bool {
        self.next.eq(&other.next)
    }
}

impl Eq for SortedSortedSegments {}
