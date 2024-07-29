use necsim_core::cogs::MathsCore;

#[derive(Clone, Debug)]
pub enum NvptxMathsCore {}

impl MathsCore for NvptxMathsCore {
    #[inline]
    fn floor(x: f64) -> f64 {
        // IEEE-compliant implementation on CPU and GPU
        unsafe { core::intrinsics::floorf64(x) }
    }

    #[inline]
    fn ceil(x: f64) -> f64 {
        // IEEE-compliant implementation on CPU and GPU
        unsafe { core::intrinsics::ceilf64(x) }
    }

    #[inline]
    fn ln(x: f64) -> f64 {
        // Guard against usage on the CPU as results will NOT match

        #[cfg(target_os = "cuda")]
        unsafe {
            const FRAC_1_LOG2_E: f64 = 1.0_f64 / core::f64::consts::LOG2_E;

            #[expect(clippy::cast_possible_truncation)]
            let x: f32 = x as f32;
            let f: f32;

            core::arch::asm!("lg2.approx.f32 {}, {};", out(reg32) f, in(reg32) x, options(pure, nomem, nostack));

            // f / log_2(e)
            f64::from(f) * FRAC_1_LOG2_E
        }
        #[cfg(not(target_os = "cuda"))]
        {
            // extern "C" {
            //     fn nvptx_maths_core_ln_on_cpu(_x: f64) -> !;
            // }

            // unsafe { nvptx_maths_core_ln_on_cpu(x) }

            // TODO: disallow using NvptxMathsCore::ln on CPU
            unsafe { core::intrinsics::logf64(x) }
        }
    }

    #[inline]
    fn exp(x: f64) -> f64 {
        // Guard against usage on the CPU as results will NOT match

        #[cfg(target_os = "cuda")]
        unsafe {
            #[expect(clippy::cast_possible_truncation)]
            let x: f32 = (x * core::f64::consts::LOG2_E) as f32;
            let f: f32;

            core::arch::asm!("ex2.approx.f32 {}, {};", out(reg32) f, in(reg32) x, options(pure, nomem, nostack));

            f64::from(f)
        }
        #[cfg(not(target_os = "cuda"))]
        {
            extern "C" {
                fn nvptx_maths_core_exp_on_cpu(_x: f64) -> !;
            }

            unsafe { nvptx_maths_core_exp_on_cpu(x) }
        }
    }

    #[inline]
    fn sqrt(x: f64) -> f64 {
        // IEEE-compliant implementation on CPU and GPU
        unsafe { core::intrinsics::sqrtf64(x) }
    }

    #[inline]
    fn pow(x: f64, exp: f64) -> f64 {
        // Guard against usage on the CPU as results will NOT match

        #[cfg(target_os = "cuda")]
        unsafe {
            // Compute x ^ exp = 2 ^ (exp * log2(x))
            // https://stackoverflow.com/a/54273307
            // by https://stackoverflow.com/users/2341466/andars
            // Licensed under CC BY-SA 4.0
            #[expect(clippy::cast_possible_truncation)]
            let x: f32 = x as f32;
            #[expect(clippy::cast_possible_truncation)]
            let exp: f32 = exp as f32;

            let log2_x: f32;
            core::arch::asm!("lg2.approx.f32 {}, {};", out(reg32) log2_x, in(reg32) x, options(pure, nomem, nostack));

            let exp_log2_x = exp * log2_x;

            let f: f32;
            core::arch::asm!("ex2.approx.f32 {}, {};", out(reg32) f, in(reg32) exp_log2_x, options(pure, nomem, nostack));

            f64::from(f)
        }
        #[cfg(not(target_os = "cuda"))]
        {
            extern "C" {
                fn nvptx_maths_core_pow_on_cpu(_x: f64, _exp: f64) -> !;
            }

            unsafe { nvptx_maths_core_pow_on_cpu(x, exp) }
        }
    }

    #[inline]
    fn sin(x: f64) -> f64 {
        // Guard against usage on the CPU as results will NOT match

        #[cfg(target_os = "cuda")]
        unsafe {
            #[expect(clippy::cast_possible_truncation)]
            let x: f32 = x as f32;
            let f: f32;

            core::arch::asm!("sin.approx.f32 {}, {};", out(reg32) f, in(reg32) x, options(pure, nomem, nostack));

            f64::from(f)
        }
        #[cfg(not(target_os = "cuda"))]
        {
            extern "C" {
                fn nvptx_maths_core_sin_on_cpu(_x: f64) -> !;
            }

            unsafe { nvptx_maths_core_sin_on_cpu(x) }
        }
    }

    #[inline]
    fn cos(x: f64) -> f64 {
        // Guard against usage on the CPU as results will NOT match

        #[cfg(target_os = "cuda")]
        unsafe {
            #[expect(clippy::cast_possible_truncation)]
            let x: f32 = x as f32;
            let f: f32;

            core::arch::asm!("cos.approx.f32 {}, {};", out(reg32) f, in(reg32) x, options(pure, nomem, nostack));

            f64::from(f)
        }
        #[cfg(not(target_os = "cuda"))]
        {
            extern "C" {
                fn nvptx_maths_core_cos_on_cpu(_x: f64) -> !;
            }

            unsafe { nvptx_maths_core_cos_on_cpu(x) }
        }
    }

    #[inline]
    fn round(x: f64) -> f64 {
        // Implementation based on IEEE-compliant f64::trunc() on CPU and GPU
        // Logic adapted from libm (Apache 2.0 / MIT dual-licensed):
        // https://github.com/rust-lang/libm/blob/1f7b8/src/math/round.rs#L6-L8

        const ROUND_TRUNC_OFFSET: f64 = 0.5_f64 - 0.25_f64 * f64::EPSILON;

        let offset: f64;

        #[cfg(target_os = "cuda")]
        unsafe {
            core::arch::asm!("copysign.f64 {}, {}, {};", out(reg64) offset, in(reg64) x, in(reg64) ROUND_TRUNC_OFFSET, options(pure, nomem, nostack));
        }
        #[cfg(not(target_os = "cuda"))]
        unsafe {
            offset = core::intrinsics::copysignf64(ROUND_TRUNC_OFFSET, x);
        }

        unsafe { core::intrinsics::truncf64(x + offset) }
    }
}
