use core::{
    cmp::{Ord, Ordering},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use necsim_core_bond::{NonNegativeF64, PositiveF64};

use crate::{
    cogs::{
        backup::BackedUp, coalescence_sampler::CoalescenceRngSample, Backup, Habitat,
        LineageReference, MathsCore,
    },
    landscape::{IndexedLocation, Location},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, TypeLayout)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[repr(transparent)]
pub struct GlobalLineageReference(u64);

impl GlobalLineageReference {
    #[doc(hidden)]
    #[must_use]
    pub unsafe fn into_inner(self) -> u64 {
        self.0
    }

    #[doc(hidden)]
    #[must_use]
    pub unsafe fn from_inner(inner: u64) -> Self {
        Self(inner)
    }
}

impl fmt::Display for GlobalLineageReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for GlobalLineageReference {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GlobalLineageReference {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let inner = u64::deserialize(deserializer)?;

        Ok(Self(inner))
    }
}

impl<M: MathsCore, H: Habitat<M>> LineageReference<M, H> for GlobalLineageReference {}

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LineageInteraction {
    None,
    Maybe,
    Coalescence(GlobalLineageReference),
}

impl LineageInteraction {
    #[must_use]
    pub const fn is_coalescence(&self) -> bool {
        matches!(self, Self::Coalescence(_))
    }

    #[must_use]
    pub const fn parent(&self) -> Option<GlobalLineageReference> {
        match self {
            Self::Coalescence(parent) => Some(GlobalLineageReference(parent.0)),
            _ => None,
        }
    }
}

impl From<Option<GlobalLineageReference>> for LineageInteraction {
    fn from(optional_coalescence: Option<GlobalLineageReference>) -> Self {
        match optional_coalescence {
            None => Self::None,
            Some(coalescence) => Self::Coalescence(coalescence),
        }
    }
}

#[allow(clippy::module_name_repetitions)] // FIXME: use expect
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TypeLayout)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[repr(C)]
#[cfg_attr(feature = "cuda", cuda(ignore))]
#[serde(deny_unknown_fields)]
pub struct Lineage {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    #[cfg_attr(feature = "cuda", cuda(ignore))]
    #[serde(alias = "id", alias = "ref")]
    pub global_reference: GlobalLineageReference,
    #[cfg_attr(feature = "cuda", cuda(ignore))]
    #[serde(alias = "time")]
    pub last_event_time: NonNegativeF64,
    #[cfg_attr(feature = "cuda", cuda(embed))]
    #[cfg_attr(feature = "cuda", cuda(ignore))]
    #[serde(alias = "loc")]
    pub indexed_location: IndexedLocation,
}

impl Lineage {
    #[must_use]
    #[debug_ensures(
        ret.indexed_location == old(indexed_location.clone()),
        "stores the indexed_location"
    )]
    #[debug_ensures(ret.last_event_time == 0.0_f64, "starts at t_0 = 0.0")]
    pub fn new<M: MathsCore, H: Habitat<M>>(
        indexed_location: IndexedLocation,
        habitat: &H,
    ) -> Self {
        Self {
            global_reference: GlobalLineageReference(
                habitat.map_indexed_location_to_u64_injective(&indexed_location),
            ),
            last_event_time: NonNegativeF64::zero(),
            indexed_location,
        }
    }
}

impl Ord for Lineage {
    fn cmp(&self, other: &Self) -> Ordering {
        // Order `Lineage`s in lexicographical order:
        //  (1) indexed_location
        //  (2) last_event_time
        //  (3) global_reference
        (
            &self.indexed_location,
            &self.last_event_time,
            &self.global_reference,
        )
            .cmp(&(
                &other.indexed_location,
                &other.last_event_time,
                &other.global_reference,
            ))
    }
}

impl PartialOrd for Lineage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(i8)]
pub enum TieBreaker {
    PreferImmigrant = -1,
    PreferLocal = 1,
}

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub struct MigratingLineage {
    pub global_reference: GlobalLineageReference,
    pub prior_time: NonNegativeF64,
    pub event_time: PositiveF64,
    pub coalescence_rng_sample: CoalescenceRngSample,
    pub dispersal_target: Location,
    pub dispersal_origin: IndexedLocation,
    pub tie_breaker: TieBreaker,
}

#[contract_trait]
impl Backup for MigratingLineage {
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            global_reference: self.global_reference.backup_unchecked(),
            dispersal_origin: self.dispersal_origin.clone(),
            dispersal_target: self.dispersal_target.clone(),
            prior_time: self.prior_time,
            event_time: self.event_time,
            coalescence_rng_sample: self.coalescence_rng_sample.backup_unchecked(),
            tie_breaker: self.tie_breaker,
        }
    }
}

impl Ord for MigratingLineage {
    fn cmp(&self, other: &Self) -> Ordering {
        // Order `MigratingLineage`s in lexicographical order:
        //  (1) event_time                       /=\
        //  (2) dispersal_origin        different | events
        //  (3) dispersal_target                 \=/
        //  (4) prior_time              parent + offspring
        //  (5) global_lineage_reference
        //  (6) coalescence_rng_sample
        // (tie_breaker is ignored as it cannot compare MigratingLineages)
        match self.event_time.cmp(&other.event_time) {
            Ordering::Equal => match (&self.dispersal_origin, &self.dispersal_target)
                .cmp(&(&other.dispersal_origin, &other.dispersal_target))
            {
                Ordering::Equal => match self.prior_time.cmp(&other.prior_time) {
                    Ordering::Equal => (&self.global_reference, &self.coalescence_rng_sample)
                        .cmp(&(&other.global_reference, &other.coalescence_rng_sample)),
                    ordering => ordering,
                },
                ordering => ordering,
            },
            ordering => ordering,
        }
    }
}

impl PartialOrd for MigratingLineage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for MigratingLineage {}

impl PartialEq<BackedUp<Self>> for MigratingLineage {
    fn eq(&self, other: &BackedUp<Self>) -> bool {
        self.eq(&**other)
    }
}
