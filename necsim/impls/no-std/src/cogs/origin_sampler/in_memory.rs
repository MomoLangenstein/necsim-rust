use core::{
    convert::TryFrom,
    fmt,
    iter::{Iterator, Peekable},
};

use necsim_core::{
    cogs::{Habitat, MathsCore},
    landscape::{IndexedLocation, LocationIterator},
    lineage::Lineage,
};

use crate::cogs::{
    habitat::in_memory::InMemoryHabitat, origin_sampler::pre_sampler::OriginPreSampler,
};

use super::{TrustedOriginSampler, UntrustedOriginSampler};

#[allow(clippy::module_name_repetitions)]
pub struct InMemoryOriginSampler<'h, M: MathsCore, I: Iterator<Item = u64>> {
    pre_sampler: OriginPreSampler<M, I>,
    last_index: u64,
    location_iterator: Peekable<LocationIterator>,
    next_location_index: u32,
    habitat: &'h InMemoryHabitat<M>,
}

impl<'h, M: MathsCore, I: Iterator<Item = u64>> fmt::Debug for InMemoryOriginSampler<'h, M, I> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(InMemoryOriginSampler))
            .field("pre_sampler", &self.pre_sampler)
            .field("last_index", &self.last_index)
            .field("location_iterator", &self.location_iterator)
            .field("next_location_index", &self.next_location_index)
            .field("habitat", &self.habitat)
            .finish()
    }
}

impl<'h, M: MathsCore, I: Iterator<Item = u64>> InMemoryOriginSampler<'h, M, I> {
    #[must_use]
    pub fn new(pre_sampler: OriginPreSampler<M, I>, habitat: &'h InMemoryHabitat<M>) -> Self {
        Self {
            pre_sampler,
            last_index: 0_u64,
            location_iterator: habitat.get_extent().iter().peekable(),
            next_location_index: 0_u32,
            habitat,
        }
    }
}

#[contract_trait]
impl<'h, M: MathsCore, I: Iterator<Item = u64>> UntrustedOriginSampler<'h, M>
    for InMemoryOriginSampler<'h, M, I>
{
    type Habitat = InMemoryHabitat<M>;
    type PreSampler = I;

    fn habitat(&self) -> &'h Self::Habitat {
        self.habitat
    }

    fn into_pre_sampler(self) -> OriginPreSampler<M, Self::PreSampler> {
        self.pre_sampler
    }

    fn full_upper_bound_size_hint(&self) -> u64 {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        {
            (f64::from(self.habitat.get_total_habitat())
                * self.pre_sampler.get_sample_proportion().get()) as u64
        }
    }
}

unsafe impl<'h, M: MathsCore, I: Iterator<Item = u64>> TrustedOriginSampler<'h, M>
    for InMemoryOriginSampler<'h, M, I>
{
}

impl<'h, M: MathsCore, I: Iterator<Item = u64>> Iterator for InMemoryOriginSampler<'h, M, I> {
    type Item = Lineage;

    fn next(&mut self) -> Option<Self::Item> {
        let next_index = self.pre_sampler.next()?;
        let mut index_difference = next_index - self.last_index;
        self.last_index = next_index;

        while u64::from(self.next_location_index) + index_difference
            >= u64::from(
                self.habitat
                    .get_habitat_at_location(self.location_iterator.peek()?),
            )
        {
            index_difference -= u64::from(
                self.habitat
                    .get_habitat_at_location(self.location_iterator.peek()?)
                    - self.next_location_index,
            );

            self.next_location_index = 0;

            self.location_iterator.next();
        }

        let next_location = self.location_iterator.peek()?;

        self.next_location_index += u32::try_from(index_difference).unwrap();

        Some(Lineage::new(
            IndexedLocation::new(next_location.clone(), self.next_location_index),
            self.habitat,
        ))
    }
}
