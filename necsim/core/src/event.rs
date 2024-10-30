use necsim_core_bond::{NonNegativeF64, PositiveF64};
use serde::{Deserialize, Serialize};

use core::{
    cmp::{Ord, Ordering},
    hash::{Hash, Hasher},
};

use crate::{
    landscape::IndexedLocation,
    lineage::{GlobalLineageReference, LineageInteraction},
};

#[expect(clippy::module_name_repetitions)]
#[expect(clippy::unsafe_derive_deserialize)]
#[derive(Debug, Clone, Serialize, Deserialize, TypeLayout)]
#[repr(C)]
pub struct PackedEvent {
    global_lineage_reference: GlobalLineageReference,
    // p\e (-) (+)
    // (-)  N   S
    // (+)  M   C
    prior_time: f64, // time of the previous event, NonNegativeF64
    event_time: f64, // time of this event, PositiveF64
    origin: IndexedLocation,
    target: IndexedLocation,
    coalescence: GlobalLineageReference,
}

impl PackedEvent {
    #[must_use]
    #[inline]
    pub fn event_time(&self) -> PositiveF64 {
        unsafe { PositiveF64::new_unchecked(self.event_time.make_positive()) }
    }
}

#[expect(dead_code)]
const EXCESSIVE_PACKED_EVENT_ERROR: [(); 1 - {
    const ASSERT: bool = core::mem::size_of::<PackedEvent>() == 56;
    ASSERT
} as usize] = [];

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum EventType {
    Speciation,
    Dispersal(Dispersal),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dispersal {
    pub target: IndexedLocation,
    pub interaction: LineageInteraction,
}

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Serialize, Deserialize, TypeLayout)]
#[repr(C)]
pub struct SpeciationEvent {
    pub global_lineage_reference: GlobalLineageReference,
    pub prior_time: NonNegativeF64, // time of the previous event
    pub event_time: PositiveF64,    // time of this event
    pub origin: IndexedLocation,
}

#[expect(dead_code)]
const EXCESSIVE_SPECIATION_EVENT_ERROR: [(); 1 - {
    const ASSERT: bool = core::mem::size_of::<SpeciationEvent>() == 40;
    ASSERT
} as usize] = [];

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispersalEvent {
    pub global_lineage_reference: GlobalLineageReference,
    pub prior_time: NonNegativeF64, // time of the previous event
    pub event_time: PositiveF64,    // time of this event
    pub origin: IndexedLocation,
    pub target: IndexedLocation,
    pub interaction: LineageInteraction,
}

#[expect(dead_code)]
const EXCESSIVE_DISPERSAL_EVENT_ERROR: [(); 1 - {
    const ASSERT: bool = core::mem::size_of::<DispersalEvent>() == 64;
    ASSERT
} as usize] = [];

#[expect(dead_code)]
const EXCESSIVE_OPTION_DISPERSAL_EVENT_ERROR: [(); 1 - {
    const ASSERT: bool =
        core::mem::size_of::<Option<DispersalEvent>>() == core::mem::size_of::<DispersalEvent>();
    ASSERT
} as usize] = [];

#[expect(clippy::module_name_repetitions)]
pub enum TypedEvent {
    Speciation(SpeciationEvent),
    Dispersal(DispersalEvent),
}

impl From<SpeciationEvent> for PackedEvent {
    fn from(event: SpeciationEvent) -> Self {
        // speciation is encoded as p(-), e(+)
        Self {
            global_lineage_reference: event.global_lineage_reference.clone(),
            prior_time: event.prior_time.get().make_negative(),
            event_time: event.event_time.get(),
            origin: event.origin.clone(),
            target: event.origin,
            coalescence: event.global_lineage_reference,
        }
    }
}

impl From<DispersalEvent> for PackedEvent {
    fn from(
        DispersalEvent {
            global_lineage_reference,
            prior_time,
            event_time,
            origin,
            target,
            interaction,
        }: DispersalEvent,
    ) -> Self {
        let prior_time = prior_time.get();
        let event_time = event_time.get();

        let (coalescence, prior_time, event_time) = match interaction {
            LineageInteraction::None => {
                // dispersal, no coalescence is encoded as p(-), e(-)
                (
                    global_lineage_reference.clone(),
                    prior_time.make_negative(),
                    event_time.make_negative(),
                )
            },
            LineageInteraction::Maybe => {
                // dispersal, maybe coalescence is encoded as p(+), e(-)
                (
                    global_lineage_reference.clone(),
                    prior_time,
                    event_time.make_negative(),
                )
            },
            LineageInteraction::Coalescence(coalescence) => {
                // dispersal, with coalescence is encoded as p(+), e(+)
                (coalescence, prior_time, event_time)
            },
        };

        Self {
            global_lineage_reference,
            prior_time,
            event_time,
            origin,
            target,
            coalescence,
        }
    }
}

impl From<TypedEvent> for PackedEvent {
    fn from(event: TypedEvent) -> Self {
        match event {
            TypedEvent::Speciation(event) => event.into(),
            TypedEvent::Dispersal(event) => event.into(),
        }
    }
}

impl From<SpeciationEvent> for TypedEvent {
    fn from(event: SpeciationEvent) -> Self {
        Self::Speciation(event)
    }
}

impl From<DispersalEvent> for TypedEvent {
    fn from(event: DispersalEvent) -> Self {
        Self::Dispersal(event)
    }
}

impl From<PackedEvent> for TypedEvent {
    fn from(
        PackedEvent {
            global_lineage_reference,
            prior_time,
            event_time,
            origin,
            target,
            coalescence,
        }: PackedEvent,
    ) -> Self {
        let prior_pos = prior_time.is_sign_positive();
        let event_pos = event_time.is_sign_positive();

        let prior_time = unsafe { NonNegativeF64::new_unchecked(prior_time.make_positive()) };
        let event_time = unsafe { PositiveF64::new_unchecked(event_time.make_positive()) };

        match (prior_pos, event_pos) {
            // dispersal, no coalescence is encoded as p(-), e(-)
            (false, false) => Self::Dispersal(DispersalEvent {
                global_lineage_reference,
                prior_time,
                event_time,
                origin,
                target,
                interaction: LineageInteraction::None,
            }),
            // speciation is encoded as p(-), e(+)
            (false, true) => Self::Speciation(SpeciationEvent {
                global_lineage_reference,
                prior_time,
                event_time,
                origin,
            }),
            // dispersal, maybe coalescence is encoded as p(+), e(-)
            (true, false) => Self::Dispersal(DispersalEvent {
                global_lineage_reference,
                prior_time,
                event_time,
                origin,
                target,
                interaction: LineageInteraction::Maybe,
            }),
            // dispersal, with coalescence is encoded as p(+), e(+)
            (true, true) => Self::Dispersal(DispersalEvent {
                global_lineage_reference,
                prior_time,
                event_time,
                origin,
                target,
                interaction: LineageInteraction::Coalescence(coalescence),
            }),
        }
    }
}

impl Eq for PackedEvent {}

impl PartialEq for PackedEvent {
    // `Event`s are equal when they have the same `origin`, `event_time`,
    //  and `target`
    // (`global_lineage_reference`, `prior_time`, and `coalescence` are ignored)
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin
            && unsafe { PositiveF64::new_unchecked(self.event_time) }
                == unsafe { PositiveF64::new_unchecked(other.event_time) }
            && self.target == other.target
    }
}

impl Ord for PackedEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // Order `Event`s in lexicographical order:
        //  (1) event_time                       /=\
        //  (2) origin                  different | events
        //  (3) target                           \=/
        //  (4) prior_time              parent + offspring
        //  (5) global_lineage_reference
        // (coalescence is ignored)
        (
            &unsafe { PositiveF64::new_unchecked(self.event_time.make_positive()) },
            &self.origin,
            &self.target,
            &unsafe { NonNegativeF64::new_unchecked(self.prior_time.make_positive()) },
            &self.global_lineage_reference,
        )
            .cmp(&(
                &unsafe { PositiveF64::new_unchecked(other.event_time.make_positive()) },
                &other.origin,
                &other.target,
                &unsafe { NonNegativeF64::new_unchecked(other.prior_time.make_positive()) },
                &other.global_lineage_reference,
            ))
    }
}

impl PartialOrd for PackedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for PackedEvent {
    // `Event`s are equal when they have the same `origin`, `event_time`,
    //  and `target`.
    // (`global_lineage_reference`, `prior_time`, and `coalescence` are ignored)
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.origin.hash(state);
        unsafe { PositiveF64::new_unchecked(self.event_time.make_positive()) }.hash(state);
        self.target.hash(state);
    }
}

impl Eq for SpeciationEvent {}

impl PartialEq for SpeciationEvent {
    // `SpeciationEvent`s are equal when they have the same `origin` and
    //  `event_time` (`global_lineage_reference` and `prior_time` are ignored)
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin && self.event_time == other.event_time
    }
}

impl Eq for DispersalEvent {}

impl PartialEq for DispersalEvent {
    // `SpeciationEvent`s are equal when they have the same `origin`,
    //  `event_time`, and `target`
    // (`global_lineage_reference`, `prior_time`, and `interaction` are ignored)
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin
            && self.event_time == other.event_time
            && self.target == other.target
    }
}

trait F64 {
    fn make_positive(self) -> Self;
    fn make_negative(self) -> Self;
}

impl F64 for f64 {
    fn make_positive(self) -> Self {
        f64::from_bits(self.to_bits() & 0x7fff_ffff_ffff_ffff_u64)
    }

    fn make_negative(self) -> Self {
        f64::from_bits(self.to_bits() | 0x8000_0000_0000_0000_u64)
    }
}

#[cfg(test)]
mod tests {
    use super::F64;

    #[test]
    fn test_make_positive() {
        assert_eq!((-0.0_f64).make_positive().to_bits(), (0.0_f64).to_bits());
        assert_eq!((-42.0_f64).make_positive().to_bits(), (42.0_f64).to_bits());
        assert_eq!((24.0_f64).make_positive().to_bits(), (24.0_f64).to_bits());
    }

    #[test]
    fn test_make_negative() {
        assert_eq!((0.0_f64).make_negative().to_bits(), (-0.0_f64).to_bits());
        assert_eq!((42.0_f64).make_negative().to_bits(), (-42.0_f64).to_bits());
        assert_eq!((-24.0_f64).make_negative().to_bits(), (-24.0_f64).to_bits());
    }
}
