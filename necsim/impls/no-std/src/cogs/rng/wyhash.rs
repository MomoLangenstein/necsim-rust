use core::marker::PhantomData;

use necsim_core::cogs::{MathsCore, PrimeableRng, RngCore};

use serde::{Deserialize, Serialize};

// WyHash constants
// https://docs.rs/wyhash/0.5.0/src/wyhash/functions.rs.html
const P0: u64 = 0xa076_1d64_78bd_642f;
const P1: u64 = 0xe703_7ed1_a0b4_28db;
const P2: u64 = 0x8ebc_6af0_9c88_c6e3;
const P5: u64 = 0xeb44_acca_b455_d165;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Serialize, Deserialize, TypeLayout)]
#[layout(free = "M")]
#[serde(deny_unknown_fields)]
#[repr(C)]
pub struct WyHash<M: MathsCore> {
    seed: u64,
    state: u64,
    #[serde(skip)]
    marker: PhantomData<M>,
}

impl<M: MathsCore> Clone for WyHash<M> {
    fn clone(&self) -> Self {
        Self {
            seed: self.seed,
            state: self.state,
            marker: PhantomData::<M>,
        }
    }
}

impl<M: MathsCore> RngCore<M> for WyHash<M> {
    type Seed = [u8; 8];

    #[must_use]
    #[inline]
    fn from_seed(seed: Self::Seed) -> Self {
        let seed = u64::from_le_bytes(seed);

        Self {
            seed,
            state: seed,
            marker: PhantomData::<M>,
        }
    }

    #[must_use]
    #[inline]
    fn sample_u64(&mut self) -> u64 {
        // wyrng state transition function
        // https://docs.rs/wyhash/0.5.0/src/wyhash/functions.rs.html#129-132
        self.state = self.state.wrapping_add(P0);

        // wyrng output function
        let wyrng = wymum(self.state ^ P1, self.state);

        // SeaHash diffusion function for better avalanching
        seahash_diffuse(wyrng)
    }
}

impl<M: MathsCore> PrimeableRng<M> for WyHash<M> {
    #[inline]
    fn prime_with(&mut self, location_index: u64, time_index: u64) {
        let location_index = seahash_diffuse(location_index);
        let time_index = seahash_diffuse(time_index);

        // wyhash state repriming
        // https://docs.rs/wyhash/0.5.0/src/wyhash/functions.rs.html#67-70
        let hash = wymum(
            location_index.rotate_right(32) ^ (self.seed ^ P0),
            time_index.rotate_right(32) ^ P2,
        );

        self.state = wymum(hash, 16 ^ P5);
    }
}

#[inline]
#[allow(clippy::cast_possible_truncation)]
fn wymum(mut a: u64, mut b: u64) -> u64 {
    // WyHash diffusion function
    // https://docs.rs/wyhash/0.5.0/src/wyhash/functions.rs.html#8-12
    let r = u128::from(a) * u128::from(b);

    // WyHash condom
    // https://github.com/wangyi-fudan/wyhash/blob/master/wyhash.h#L57
    a ^= r as u64;
    b ^= (r >> 64) as u64;

    a ^ b
}

#[inline]
const fn seahash_diffuse(mut x: u64) -> u64 {
    // SeaHash diffusion function
    // https://docs.rs/seahash/4.1.0/src/seahash/helper.rs.html#75-92

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
