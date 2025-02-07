use necsim_core_maths::MathsCore;

use super::{vector::VecType, NoiseEvaluator, PermTable};

pub fn contribute<NoiseEvaluatorType: NoiseEvaluator<Vec>, Vec: VecType<f64>, M: MathsCore>(
    delta: Vec,
    origin: Vec,
    grid: Vec,
    perm: &PermTable,
    wrap: f64,
) -> f64 {
    let shifted: Vec = origin - delta - NoiseEvaluatorType::SQUISH_POINT * delta.sum();
    let attn: f64 = 2.0 - shifted.get_attenuation_factor();

    if attn <= 0.0 {
        return 0.0;
    }

    let attn2 = attn * attn;
    let attn4 = attn2 * attn2;

    attn4 * NoiseEvaluatorType::extrapolate::<M>(grid + delta, shifted, perm, wrap)
}
