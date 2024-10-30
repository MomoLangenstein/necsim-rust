use necsim_core_bond::{NonNegativeF64, PositiveF64};

pub mod indexed;
pub mod stack;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq)]
struct PositiveF64Decomposed {
    exponent: i16,
    mantissa: u64,
}

fn decompose_weight(weight: PositiveF64) -> PositiveF64Decomposed {
    let bits = weight.get().to_bits();

    let mut exponent: i16 = ((bits >> 52) & 0x7ff_u64) as i16;

    let mantissa = if exponent == 0 {
        // Ensure that subnormal floats are presented internally as if they were normal
        #[expect(clippy::cast_possible_truncation)]
        let subnormal_exponent = (bits.leading_zeros() as i16) - 12;
        exponent -= subnormal_exponent;

        // weight > 0 && exponent == 0 -> all bits before mantissa are zero
        bits << (bits.leading_zeros() - 11)
    } else {
        // Add the implicit 1.x to the 0.x mantissa
        (bits & 0x000f_ffff_ffff_ffff_u64) | 0x0010_0000_0000_0000_u64
    };

    PositiveF64Decomposed {
        exponent: exponent - 1023,
        mantissa,
    }
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn compose_weight(mut exponent: i16, mut mantissa: u128) -> NonNegativeF64 {
    if mantissa == 0 {
        return NonNegativeF64::zero();
    }

    let mut excess_exponent = 75 - (mantissa.leading_zeros() as i16);

    // Round up if the most significant bit being cut off is 1
    if excess_exponent > 0 && (mantissa & (1_u128 << (excess_exponent - 1))) != 0 {
        mantissa += 1_u128 << excess_exponent;
    }

    excess_exponent = 75 - (mantissa.leading_zeros() as i16);
    exponent += excess_exponent;

    let bits = if exponent >= -1022 {
        // Only keep the 52 bit mantissa for the normal float
        let mantissa_u64 = ((mantissa >> excess_exponent) & 0x000f_ffff_ffff_ffff_u128) as u64;

        (((exponent + 1023) as u64) << 52) | mantissa_u64
    } else {
        // Reconstruct a subnormal float mantissa, its encoded exponent is 0
        let mantissa_u64 =
            ((mantissa >> (excess_exponent - 1022 - exponent)) & 0x000f_ffff_ffff_ffff_u128) as u64;

        #[allow(clippy::let_and_return)] // FIXME: use expect
        {
            mantissa_u64
        }
    };

    unsafe { NonNegativeF64::new_unchecked(f64::from_bits(bits)) }
}
