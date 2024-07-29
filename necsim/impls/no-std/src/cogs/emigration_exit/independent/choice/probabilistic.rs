use necsim_core::{
    cogs::{Backup, Habitat, MathsCore},
    landscape::IndexedLocation,
};
use necsim_core_bond::{ClosedUnitF64, PositiveF64};

use super::EmigrationChoice;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct ProbabilisticEmigrationChoice {
    probability: ClosedUnitF64,
}

impl ProbabilisticEmigrationChoice {
    #[must_use]
    pub fn new(probability: ClosedUnitF64) -> Self {
        Self { probability }
    }
}

#[contract_trait]
impl Backup for ProbabilisticEmigrationChoice {
    unsafe fn backup_unchecked(&self) -> Self {
        Self {
            probability: self.probability,
        }
    }
}

#[contract_trait]
impl<M: MathsCore, H: Habitat<M>> EmigrationChoice<M, H> for ProbabilisticEmigrationChoice {
    fn should_lineage_emigrate(
        &self,
        _indexed_location: &IndexedLocation,
        time: PositiveF64,
        _habitat: &H,
    ) -> bool {
        let hash = diffuse(time.get().to_bits());

        // http://prng.di.unimi.it -> Generating uniform doubles in the unit interval
        #[expect(clippy::cast_precision_loss)]
        let u01 = ((hash >> 11) as f64) * f64::from_bits(0x3CA0_0000_0000_0000_u64); // 0x1.0p-53

        u01 <= self.probability.get()
    }
}

// https://docs.rs/seahash/4.0.1/src/seahash/helper.rs.html#72-89
#[inline]
const fn diffuse(mut x: u64) -> u64 {
    // These are derived from the PCG RNG's round. Thanks to @Veedrac for proposing
    // this. The basic idea is that we use dynamic shifts, which are determined
    // by the input itself. The shift is chosen by the higher bits, which means
    // that changing those flips the lower bits, which scatters upwards because
    // of the multiplication.

    x = x.wrapping_mul(0x6eed_0e9d_a4d9_4a4f);

    let a = x >> 32;
    let b = x >> 60;

    x ^= a >> b;

    x = x.wrapping_mul(0x6eed_0e9d_a4d9_4a4f);

    x
}
