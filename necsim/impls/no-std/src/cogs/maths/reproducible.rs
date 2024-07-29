use necsim_core::cogs::MathsCore;

#[derive(Clone, Debug)]
#[expect(clippy::module_name_repetitions)]
pub enum ReproducibleMathsCore {}

impl MathsCore for ReproducibleMathsCore {
    #[inline]
    fn floor(x: f64) -> f64 {
        libm::floor(x)
    }

    #[inline]
    fn ceil(x: f64) -> f64 {
        libm::ceil(x)
    }

    #[inline]
    fn ln(x: f64) -> f64 {
        libm::log(x)
    }

    #[inline]
    fn exp(x: f64) -> f64 {
        libm::exp(x)
    }

    #[inline]
    fn sqrt(x: f64) -> f64 {
        libm::sqrt(x)
    }

    #[inline]
    fn pow(x: f64, exp: f64) -> f64 {
        libm::pow(x, exp)
    }

    #[inline]
    fn sin(x: f64) -> f64 {
        libm::sin(x)
    }

    #[inline]
    fn cos(x: f64) -> f64 {
        libm::cos(x)
    }

    #[inline]
    fn round(x: f64) -> f64 {
        libm::round(x)
    }
}
