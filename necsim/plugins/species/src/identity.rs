use std::{
    array::TryFromSliceError,
    convert::{TryFrom, TryInto},
    ops::Deref,
};

use necsim_core::{
    landscape::{IndexedLocation, Location},
    lineage::GlobalLineageReference,
};
use necsim_core_bond::PositiveF64;

#[expect(clippy::module_name_repetitions)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SpeciesIdentity([u8; 24]);

impl Deref for SpeciesIdentity {
    type Target = [u8; 24];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<&[u8]> for SpeciesIdentity {
    type Error = TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl From<[u8; 24]> for SpeciesIdentity {
    fn from(value: [u8; 24]) -> Self {
        Self(value)
    }
}

impl SpeciesIdentity {
    pub fn from_speciation(origin: &IndexedLocation, time: PositiveF64) -> SpeciesIdentity {
        let location = (u64::from(origin.location().y()) << 32) | u64::from(origin.location().x());
        let index = (u64::from(origin.index()) + 1) << 16;
        let time = time.get().to_bits();

        Self::from_raw(location, index, time)
    }

    pub fn from_unspeciated(
        lineage: GlobalLineageReference,
        anchor: GlobalLineageReference,
    ) -> SpeciesIdentity {
        let lineage = unsafe { lineage.into_inner() };
        let marker = 0x0;
        let anchor = unsafe { anchor.into_inner() };

        Self::from_raw(lineage, marker, anchor)
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub fn try_into_speciation(self) -> Result<(IndexedLocation, PositiveF64), Self> {
        let (location, index, time) = self.copy_into_raw();

        let index = index.wrapping_sub(1 << 16);

        if index & 0xFFFF_0000_0000_FFFF_u64 != 0x0 {
            return Err(self);
        }

        #[expect(clippy::cast_possible_truncation)]
        let x = (location & u64::from(u32::MAX)) as u32;
        let y = ((location >> 32) & u64::from(u32::MAX)) as u32;
        #[expect(clippy::cast_possible_truncation)]
        let i = ((index >> 16) & u64::from(u32::MAX)) as u32;

        let origin = IndexedLocation::new(Location::new(x, y), i);

        let Ok(time) = PositiveF64::new(f64::from_bits(time)) else {
            return Err(self);
        };

        Ok((origin, time))
    }

    pub fn try_into_unspeciated(
        self,
    ) -> Result<(GlobalLineageReference, GlobalLineageReference), Self> {
        let (lineage, marker, anchor) = self.copy_into_raw();

        if marker != 0x0 {
            return Err(self);
        }

        let lineage = unsafe { GlobalLineageReference::from_inner(lineage) };
        let anchor = unsafe { GlobalLineageReference::from_inner(anchor) };

        Ok((lineage, anchor))
    }

    const fn from_raw(a: u64, b: u64, c: u64) -> Self {
        let a_bytes = seahash_diffuse(a).to_le_bytes();
        let b_bytes = seahash_diffuse(b).to_le_bytes();
        let c_bytes = seahash_diffuse(c).to_le_bytes();

        // Shuffle and mix all 24 bytes of the species identity
        let lower = seahash_diffuse(u64::from_le_bytes([
            a_bytes[3], c_bytes[0], b_bytes[5], a_bytes[1], c_bytes[4], c_bytes[7], c_bytes[5],
            a_bytes[5],
        ]))
        .to_le_bytes();
        let middle = seahash_diffuse(u64::from_le_bytes([
            c_bytes[6], b_bytes[4], a_bytes[0], a_bytes[6], b_bytes[2], b_bytes[1], a_bytes[7],
            b_bytes[3],
        ]))
        .to_le_bytes();
        let upper = seahash_diffuse(u64::from_le_bytes([
            a_bytes[4], a_bytes[2], c_bytes[2], b_bytes[0], c_bytes[3], c_bytes[1], b_bytes[7],
            b_bytes[6],
        ]))
        .to_le_bytes();

        Self([
            lower[0], lower[1], lower[2], lower[3], lower[4], lower[5], lower[6], lower[7],
            middle[0], middle[1], middle[2], middle[3], middle[4], middle[5], middle[6], middle[7],
            upper[0], upper[1], upper[2], upper[3], upper[4], upper[5], upper[6], upper[7],
        ])
    }

    const fn copy_into_raw(&self) -> (u64, u64, u64) {
        let lower_bytes = seahash_undiffuse(u64::from_le_bytes([
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7],
        ]))
        .to_le_bytes();
        let middle_bytes = seahash_undiffuse(u64::from_le_bytes([
            self.0[8], self.0[9], self.0[10], self.0[11], self.0[12], self.0[13], self.0[14],
            self.0[15],
        ]))
        .to_le_bytes();
        let upper_bytes = seahash_undiffuse(u64::from_le_bytes([
            self.0[16], self.0[17], self.0[18], self.0[19], self.0[20], self.0[21], self.0[22],
            self.0[23],
        ]))
        .to_le_bytes();

        let a = seahash_undiffuse(u64::from_le_bytes([
            middle_bytes[2],
            lower_bytes[3],
            upper_bytes[1],
            lower_bytes[0],
            upper_bytes[0],
            lower_bytes[7],
            middle_bytes[3],
            middle_bytes[6],
        ]));
        let b = seahash_undiffuse(u64::from_le_bytes([
            upper_bytes[3],
            middle_bytes[5],
            middle_bytes[4],
            middle_bytes[7],
            middle_bytes[1],
            lower_bytes[2],
            upper_bytes[7],
            upper_bytes[6],
        ]));
        let c = seahash_undiffuse(u64::from_le_bytes([
            lower_bytes[1],
            upper_bytes[5],
            upper_bytes[2],
            upper_bytes[4],
            lower_bytes[4],
            lower_bytes[6],
            middle_bytes[0],
            lower_bytes[5],
        ]));

        (a, b, c)
    }
}

const fn seahash_diffuse(mut x: u64) -> u64 {
    // SeaHash diffusion function
    // https://docs.rs/seahash/4.1.0/src/seahash/helper.rs.html#75-92

    // These are derived from the PCG RNG's round. Thanks to @Veedrac for proposing
    // this. The basic idea is that we use dynamic shifts, which are determined
    // by the input itself. The shift is chosen by the higher bits, which means
    // that changing those flips the lower bits, which scatters upwards because
    // of the multiplication.

    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);

    x = x.wrapping_mul(0x6eed_0e9d_a4d9_4a4f);

    let a = x >> 32;
    let b = x >> 60;

    x ^= a >> b;

    x = x.wrapping_mul(0x6eed_0e9d_a4d9_4a4f);

    x
}

const fn seahash_undiffuse(mut x: u64) -> u64 {
    // SeaHash undiffusion function
    // https://docs.rs/seahash/4.1.0/src/seahash/helper.rs.html#94-105

    // 0x2f72b4215a3d8caf is the modular multiplicative inverse of the constant used
    // in `diffuse`.

    x = x.wrapping_mul(0x2f72_b421_5a3d_8caf);

    let a = x >> 32;
    let b = x >> 60;

    x ^= a >> b;

    x = x.wrapping_mul(0x2f72_b421_5a3d_8caf);

    x = x.wrapping_sub(0x9e37_79b9_7f4a_7c15);

    x
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, RngCore, SeedableRng};

    use necsim_core::{
        landscape::{IndexedLocation, Location},
        lineage::GlobalLineageReference,
    };
    use necsim_core_bond::PositiveF64;

    use super::SpeciesIdentity;

    #[test]
    fn test_species_identity_from_speciation() {
        let mut rng = StdRng::from_entropy();

        for _ in 0..1_000_000 {
            let x = rng.next_u32();
            let y = rng.next_u32();
            let i = rng.next_u32();
            let origin = IndexedLocation::new(Location::new(x, y), i);

            let time = loop {
                let t = f64::from_bits(rng.next_u64());

                if t.is_finite() && t > 0.0_f64 {
                    break PositiveF64::new(t).unwrap();
                }
            };

            let identity = SpeciesIdentity::from_speciation(&origin, time);

            assert_eq!(
                identity.clone().try_into_unspeciated(),
                Err(identity.clone())
            );
            assert_eq!(identity.try_into_speciation(), Ok((origin, time)));
        }
    }

    #[test]
    fn test_species_identity_from_unspeciated() {
        let mut rng = StdRng::from_entropy();

        for _ in 0..1_000_000 {
            let lineage = unsafe { GlobalLineageReference::from_inner(rng.next_u64()) };
            let anchor = unsafe { GlobalLineageReference::from_inner(rng.next_u64()) };

            let identity = SpeciesIdentity::from_unspeciated(lineage.clone(), anchor.clone());

            assert_eq!(
                identity.clone().try_into_speciation(),
                Err(identity.clone())
            );
            assert_eq!(identity.try_into_unspeciated(), Ok((lineage, anchor)));
        }
    }
}
