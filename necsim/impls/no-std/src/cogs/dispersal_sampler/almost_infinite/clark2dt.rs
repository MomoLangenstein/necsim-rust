use core::marker::PhantomData;

use necsim_core::{
    cogs::{DispersalSampler, MathsCore, RngCore, RngSampler, SeparableDispersalSampler},
    landscape::Location,
};
use necsim_core_bond::{ClosedUnitF64, NonNegativeF64, PositiveF64};

use crate::cogs::habitat::almost_infinite::AlmostInfiniteHabitat;

#[derive(Debug)]
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(feature = "cuda", cuda(free = "M", free = "G"))]
pub struct AlmostInfiniteClark2DtDispersalSampler<M: MathsCore, G: RngCore<M>> {
    shape_u: PositiveF64,
    tail_p: PositiveF64,
    self_dispersal: ClosedUnitF64,
    marker: PhantomData<(M, G)>,
}

impl<M: MathsCore, G: RngCore<M>> AlmostInfiniteClark2DtDispersalSampler<M, G> {
    #[must_use]
    pub fn new(shape_u: PositiveF64, tail_p: PositiveF64) -> Self {
        const N: i32 = 1 << 22;

        // For now, we numerically integrate the self-dispersal probability
        //  using polar coordinates
        #[expect(clippy::useless_conversion)] // prepare for new range iterators
        let self_dispersal = (0..N)
            .into_iter()
            .map(|i| {
                // phi in [0, pi/4]
                core::f64::consts::PI * 0.25 * f64::from(i) / f64::from(N)
            })
            .map(|phi| {
                // self-dispersal jump radius: dx <= 0.5 && dy <= 0.5
                // use polar coordinates to compute the angle-dependent radius of a square:
                //  1. unit circle has radius 1
                //  2. radius (hypothenuse) scales proportionally with adjacent (cos) and
                //     opposite (sin)
                //  3. we're tracing 1/8th of a square in [0, pi/4] where k*sin(x)=? and
                //     k*cos(x)=1 and r=k, so k = 1/cos(x) = r
                //  4. unit square has side length 1, so we scale the radius to 0.5
                let jump_r = 0.5 / M::cos(phi);
                // Safety: cos([0, pi/4]) in [sqrt(2)/2, 1], and its inverse is non-negative
                unsafe { NonNegativeF64::new_unchecked(jump_r) }
            })
            .map(|jump_r| {
                // probability of dispersal to a jump distance <= jump_r
                clark2dt::cdf::<M>(jump_r, shape_u, tail_p).get()
            })
            .sum::<f64>()
            / f64::from(N); // take the average

        // Safety: the average of the cdfs, which are all ClosedUnitF64,
        //         is also in [0, 1]
        // Note: we still clamp to account for rounding errors
        let self_dispersal =
            unsafe { ClosedUnitF64::new_unchecked(self_dispersal.clamp(0.0, 1.0)) };

        Self {
            shape_u,
            tail_p,
            self_dispersal,
            marker: PhantomData::<(M, G)>,
        }
    }
}

impl<M: MathsCore, G: RngCore<M>> Clone for AlmostInfiniteClark2DtDispersalSampler<M, G> {
    fn clone(&self) -> Self {
        Self {
            shape_u: self.shape_u,
            tail_p: self.tail_p,
            self_dispersal: self.self_dispersal,
            marker: PhantomData::<(M, G)>,
        }
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> DispersalSampler<M, AlmostInfiniteHabitat<M>, G>
    for AlmostInfiniteClark2DtDispersalSampler<M, G>
{
    #[must_use]
    fn sample_dispersal_from_location(
        &self,
        location: &Location,
        _habitat: &AlmostInfiniteHabitat<M>,
        rng: &mut G,
    ) -> Location {
        let jump =
            clark2dt::cdf_inverse::<M>(rng.sample_uniform_closed_open(), self.shape_u, self.tail_p);
        let theta = rng.sample_uniform_open_closed().get() * 2.0 * core::f64::consts::PI;

        let dx = M::cos(theta) * jump;
        let dy = M::sin(theta) * jump;

        AlmostInfiniteHabitat::<M>::clamp_round_dispersal(location, dx, dy)
    }
}

#[contract_trait]
impl<M: MathsCore, G: RngCore<M>> SeparableDispersalSampler<M, AlmostInfiniteHabitat<M>, G>
    for AlmostInfiniteClark2DtDispersalSampler<M, G>
{
    #[must_use]
    fn sample_non_self_dispersal_from_location(
        &self,
        location: &Location,
        habitat: &AlmostInfiniteHabitat<M>,
        rng: &mut G,
    ) -> Location {
        let mut target_location = self.sample_dispersal_from_location(location, habitat, rng);

        // For now, we just use rejection sampling here
        while &target_location == location {
            target_location = self.sample_dispersal_from_location(location, habitat, rng);
        }

        target_location
    }

    #[must_use]
    fn get_self_dispersal_probability_at_location(
        &self,
        _location: &Location,
        _habitat: &AlmostInfiniteHabitat<M>,
    ) -> ClosedUnitF64 {
        self.self_dispersal
    }
}

#[expect(clippy::doc_markdown, clippy::module_inception)]
/// Clark2Dt dispersal:
///
/// Clark, J.S., Silman, M., Kern, R., Macklin, E. and HilleRisLambers, J.
/// (1999). Seed dispersal near and far: Patterns across temperate and tropical
/// forests. Ecology, 80: 1475-1494. Available from:
/// doi:10.1890/0012-9658(1999)080[1475:SDNAFP]2.0.CO;2
///
/// r: dispersal jump distance (radius of a circle)
/// u: distribution shape, E[X] = pi/2 * sqrt(ln(u))
/// p: distribution tail width
///
/// pdf(r) = (2 * pi * r) * p / (pi * u * (1 + r*r / u)) ** (p + 1)
/// cdf(r) = 1 - (1 / (1 + r*r / u) ** p)
/// r = cdf_inv(u01) = sqrt(u * (((1 / (1 - u01)) ** (1/p)) - 1))
///
/// See <https://gist.github.com/juntyr/c04f231ba8063a336744f1e1359f40d8>
mod clark2dt {
    use necsim_core::cogs::MathsCore;
    use necsim_core_bond::{ClosedOpenUnitF64, ClosedUnitF64, NonNegativeF64, PositiveF64};

    pub fn cdf<M: MathsCore>(
        jump_r: NonNegativeF64,
        shape_u: PositiveF64,
        tail_p: PositiveF64,
    ) -> ClosedUnitF64 {
        let u01 = 1.0
            - (1.0
                / pow_fast_one::<M>(
                    1.0 + (jump_r.get() * jump_r.get() / shape_u.get()),
                    tail_p.get(),
                ));

        unsafe { ClosedUnitF64::new_unchecked(u01) }
    }

    pub fn cdf_inverse<M: MathsCore>(
        u01: ClosedOpenUnitF64,
        shape_u: PositiveF64,
        tail_p: PositiveF64,
    ) -> f64 {
        M::sqrt(
            shape_u.get() * (pow_fast_one::<M>(1.0 / (1.0 - u01.get()), 1.0 / tail_p.get()) - 1.0),
        )
    }

    fn pow_fast_one<M: MathsCore>(x: f64, exp: f64) -> f64 {
        #[expect(clippy::float_cmp)]
        if exp == 1.0 {
            return x;
        }

        M::pow(x, exp)
    }
}

#[cfg(test)]
mod tests {
    use necsim_core::{
        cogs::{DispersalSampler, SeedableRng},
        landscape::Location,
    };
    use necsim_core_bond::{ClosedOpenUnitF64, NonNegativeF64, PositiveF64};

    use crate::cogs::{
        habitat::almost_infinite::AlmostInfiniteHabitat,
        maths::reproducible::ReproducibleMathsCore, rng::wyhash::WyHash,
    };

    use super::{
        clark2dt::{cdf, cdf_inverse},
        AlmostInfiniteClark2DtDispersalSampler,
    };

    #[test]
    fn test_self_dispersal() {
        const N: i32 = 1 << 22;

        let habitat = AlmostInfiniteHabitat::<ReproducibleMathsCore>::default();
        let origin = Location::new(0, 0);

        for shape_u in [
            PositiveF64::new(0.1).unwrap(),
            PositiveF64::new(1.0).unwrap(),
            PositiveF64::new(10.0).unwrap(),
        ] {
            for tail_p in [
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap(),
            ] {
                let dispersal = AlmostInfiniteClark2DtDispersalSampler::new(shape_u, tail_p);
                let self_dispersal = dispersal.self_dispersal;

                let mut rng = WyHash::<ReproducibleMathsCore>::seed_from_u64(42);
                let mut counter = 0_i32;

                for _ in 0..N {
                    let target =
                        dispersal.sample_dispersal_from_location(&origin, &habitat, &mut rng);

                    if target == origin {
                        counter += 1;
                    }
                }

                let self_dispersal_emperical = f64::from(counter) / f64::from(N);

                assert!(
                    (self_dispersal.get() - self_dispersal_emperical).abs() < 1e-3,
                    "{} !~ {self_dispersal_emperical} for u={}, p={}",
                    self_dispersal.get(),
                    shape_u.get(),
                    tail_p.get()
                );
            }
        }
    }

    macro_rules! assert_eq_bits {
        ($x:expr, $y:literal) => {
            assert!(
                f64::from($x).to_bits() == f64::to_bits($y),
                "{} != {}",
                f64::from($x),
                $y
            )
        };
    }

    #[test]
    #[expect(clippy::too_many_lines)]
    fn test_cdf() {
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::zero(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::zero(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::zero(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::zero(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::zero(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::zero(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::zero(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::zero(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::zero(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.0
        );

        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(0.1).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.009_485_741_785_478_23
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.000_994_538_204_051_043
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(0.1).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.000_099_945_038_470_106_16
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(0.1).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.090_909_090_909_090_94
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.009_900_990_099_009_91
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(0.1).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.000_999_000_999_000_854_2
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(0.1).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.614_456_710_570_468_6
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.094_713_045_307_016_74
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(0.1).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.009_945_219_286_995_988
        );

        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(1.0).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.213_206_557_803_227_73
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(1.0).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.066_967_008_463_192_59
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.009_485_741_785_478_23
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(1.0).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.909_090_909_090_909_1
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(1.0).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.5
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.090_909_090_909_090_94
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(1.0).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.999_999_999_961_445_6
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(1.0).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.999_023_437_5
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.614_456_710_570_468_6
        );

        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(10.0).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.498_862_857_550_073_4
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(10.0).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.369_670_166_704_018_87
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(10.0).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.213_206_557_803_227_73
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(10.0).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.999_000_999_000_999
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(10.0).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.990_099_009_900_990_1
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(10.0).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.909_090_909_090_909_1
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(10.0).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            1.0
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(10.0).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            1.0
        );
        assert_eq_bits!(
            cdf::<ReproducibleMathsCore>(
                NonNegativeF64::new(10.0).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.999_999_999_961_445_6
        );
    }

    #[test]
    #[expect(clippy::too_many_lines)]
    fn test_cdf_inverse() {
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.0).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.0).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.0).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.0).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.0).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.0).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.0).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.0).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.0
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.0).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.0
        );

        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.25).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            1.294_516_382_044_375_2
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.25).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            4.093_620_235_660_922
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.25).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            12.945_163_820_443_751
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.25).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.182_574_185_835_055_36
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.25).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            0.577_350_269_189_625_7
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.25).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            1.825_741_858_350_553_6
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.25).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.054_024_077_007_164_73
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.25).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.170_839_131_830_973_2
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.25).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.540_240_770_071_647_3
        );

        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.95).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            1_011_928.851_253_827_5
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.95).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            3_199_999.999_999_829_6
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.95).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(0.1).unwrap()
            ),
            10_119_288.512_538_275
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.95).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            1.378_404_875_209_021_7
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.95).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            4.358_898_943_540_671
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.95).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(1.0).unwrap()
            ),
            13.784_048_752_090_216
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.95).unwrap(),
                PositiveF64::new(0.1).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.186_891_104_034_826_45
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.95).unwrap(),
                PositiveF64::new(1.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            0.591_001_563_173_536_1
        );
        assert_eq_bits!(
            cdf_inverse::<ReproducibleMathsCore>(
                ClosedOpenUnitF64::new(0.95).unwrap(),
                PositiveF64::new(10.0).unwrap(),
                PositiveF64::new(10.0).unwrap()
            ),
            1.868_911_040_348_264_5
        );
    }
}
