use core::cmp::Ordering;

use alloc::vec::Vec;

use necsim_core::cogs::{MathsCore, RngCore};
use necsim_core_bond::{ClosedUnitF64, NonNegativeF64};

#[derive(Clone, Debug, TypeLayout)]
#[repr(C)]
pub struct AliasMethodSamplerAtom<E: Copy + PartialEq> {
    u: ClosedUnitF64,
    e: E,
    k: E,
}

struct AliasMethodSamplerAtomRaw<E: Copy + PartialEq> {
    u: NonNegativeF64,
    e: E,
    k: E,
}

impl<E: Copy + PartialEq> AliasMethodSamplerAtom<E> {
    pub fn u(&self) -> ClosedUnitF64 {
        self.u
    }

    pub fn e(&self) -> E {
        self.e
    }

    pub fn k(&self) -> E {
        self.k
    }

    pub fn flip(&mut self) {
        core::mem::swap(&mut self.e, &mut self.k);
        self.u = self.u.one_minus();
    }

    #[debug_requires(!event_weights.is_empty(), "event_weights is non-empty")]
    #[debug_requires(
        event_weights.iter().all(|(_e, p)| *p >= 0.0_f64),
        "all event weights are non-negative"
    )]
    #[debug_ensures(
        ret.iter().map(|s| &s.e).eq(old(event_weights).iter().map(|(e, _p)| e)),
        "stores exactly the input events"
    )]
    #[debug_ensures(
        ret.iter().all(|s| {
            let full_bucket = s.u == ClosedUnitF64::one();
            !full_bucket || (s.e == s.k)
        }),
        "full buckets sample the same event just in case"
    )]
    pub fn create(event_weights: &[(E, NonNegativeF64)]) -> Vec<AliasMethodSamplerAtom<E>> {
        let mut alias_samplers = Vec::with_capacity(event_weights.len());

        let total_weight: NonNegativeF64 = event_weights.iter().map(|(_e, p)| *p).sum();

        let n = NonNegativeF64::from(event_weights.len());

        for (event, weight) in event_weights {
            alias_samplers.push(AliasMethodSamplerAtomRaw {
                u: *weight * n / total_weight,
                e: *event,
                k: *event,
            });
        }

        let mut overfull_indices: Vec<usize> = (0..alias_samplers.len())
            .filter(|i| alias_samplers[*i].u > 1.0_f64)
            .collect();
        let mut underfull_indices: Vec<usize> = (0..alias_samplers.len())
            .filter(|i| alias_samplers[*i].u < 1.0_f64)
            .collect();

        while let Some((overfull_index, underfull_index)) =
            pop_overfull_underfull_pair_atomic(&mut overfull_indices, &mut underfull_indices)
        {
            // Safety: alias_samplers[overfull_index].U > 1.0,
            //         so (alias_samplers[overfull_index].U - 1.0) > 0.0
            alias_samplers[overfull_index].u = unsafe {
                NonNegativeF64::new_unchecked(
                    alias_samplers[overfull_index].u.get()
                        + alias_samplers[underfull_index].u.get()
                        - 1.0_f64,
                )
            };
            alias_samplers[underfull_index].k = alias_samplers[overfull_index].e;

            match alias_samplers[overfull_index].u.partial_cmp(&1.0_f64) {
                Some(Ordering::Less) => underfull_indices.push(overfull_index),
                Some(Ordering::Greater) => overfull_indices.push(overfull_index),
                _ => (),
            };
        }

        // Fix rounding errors for full indices:
        //   M. D. Vose, "A linear algorithm for generating random numbers with a given
        //   distribution", in IEEE Transactions on Software Engineering, vol. 17, no.
        // 9,   pp. 972-975, Sept. 1991, doi: 10.1109/32.92917.
        overfull_indices
            .into_iter()
            .for_each(|i| alias_samplers[i].u = NonNegativeF64::one());
        underfull_indices
            .into_iter()
            .for_each(|i| alias_samplers[i].u = NonNegativeF64::one());

        // Safety: The bucket weights are now probabilities in [0.0; 1.0]
        unsafe {
            core::mem::transmute::<Vec<AliasMethodSamplerAtomRaw<E>>, Vec<AliasMethodSamplerAtom<E>>>(
                alias_samplers,
            )
        }
    }

    pub fn sample_event<M: MathsCore, G: RngCore<M>>(
        alias_samplers: &[AliasMethodSamplerAtom<E>],
        rng: &mut G,
    ) -> E {
        Self::sample_event_with_cdf_limit(alias_samplers, rng, ClosedUnitF64::one())
    }

    #[debug_requires(!alias_samplers.is_empty(), "alias_samplers is non-empty")]
    #[debug_ensures(
        old(alias_samplers).iter().map(|s| s.e).any(|e| e == ret),
        "returns one of the weighted events"
    )]
    pub fn sample_event_with_cdf_limit<M: MathsCore, G: RngCore<M>>(
        alias_samplers: &[AliasMethodSamplerAtom<E>],
        rng: &mut G,
        limit: ClosedUnitF64,
    ) -> E {
        use necsim_core::cogs::RngSampler;

        #[expect(clippy::cast_precision_loss)]
        let f =
            rng.sample_uniform_closed_open().get() * limit.get() * (alias_samplers.len() as f64);

        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let i = M::floor(f) as usize; // index into events

        #[expect(clippy::cast_precision_loss)]
        let y = f - (i as f64); // U(0,1) to compare against U[i]

        let sample = &alias_samplers[i];

        if y < sample.u.get() {
            sample.e
        } else {
            sample.k
        }
    }
}

fn pop_overfull_underfull_pair_atomic(
    overfull_indices: &mut Vec<usize>,
    underfull_indices: &mut Vec<usize>,
) -> Option<(usize, usize)> {
    match (overfull_indices.pop(), underfull_indices.pop()) {
        (Some(overfull_index), Some(underfull_index)) => Some((overfull_index, underfull_index)),
        (Some(overfull_index), None) => {
            overfull_indices.push(overfull_index);
            None
        },
        (None, Some(underfull_index)) => {
            underfull_indices.push(underfull_index);
            None
        },
        (None, None) => None,
    }
}
