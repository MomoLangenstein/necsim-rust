#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

// OpenSimplex noise implementation vendored from
// https://github.com/Mapet13/opensimplex_noise_rust
// by Jakub Sordyl, licensed under the Unlicense
//
// Adapted to use MathsCore and support wrapping

use necsim_core_maths::MathsCore;

mod open_simplex_noise_2d;
mod open_simplex_noise_3d;
mod open_simplex_noise_4d;
mod utils;
mod vector;

use open_simplex_noise_2d::OpenSimplexNoise2D;
use open_simplex_noise_3d::OpenSimplexNoise3D;
use open_simplex_noise_4d::OpenSimplexNoise4D;
use vector::{vec2::Vec2, vec3::Vec3, vec4::Vec4};

const PSIZE: i64 = 2048;
const DEFAULT_SEED: i64 = 0;

type PermTable = [i64; PSIZE as usize];

#[derive(Clone, TypeLayout)]
#[repr(transparent)]
pub struct OpenSimplexNoise {
    perm: PermTable,
}

impl OpenSimplexNoise {
    #[must_use]
    pub fn new(custom_seed: Option<i64>) -> Self {
        let seed = match custom_seed {
            Some(value) => value,
            None => DEFAULT_SEED,
        };

        Self {
            perm: generate_perm_array(seed),
        }
    }

    #[must_use]
    pub fn eval_2d<M: MathsCore>(&self, x: f64, y: f64, wrap: f64) -> f64 {
        OpenSimplexNoise2D::eval::<M>(Vec2::new(x, y), &self.perm, wrap)
    }

    #[must_use]
    #[allow(dead_code)] // FIXME: use expect
    pub fn eval_3d<M: MathsCore>(&self, x: f64, y: f64, z: f64, wrap: f64) -> f64 {
        OpenSimplexNoise3D::eval::<M>(Vec3::new(x, y, z), &self.perm, wrap)
    }

    #[must_use]
    #[allow(dead_code)] // FIXME: use expect
    pub fn eval_4d<M: MathsCore>(&self, x: f64, y: f64, z: f64, w: f64, wrap: f64) -> f64 {
        OpenSimplexNoise4D::eval::<M>(Vec4::new(x, y, z, w), &self.perm, wrap)
    }
}

pub trait NoiseEvaluator<T: vector::VecType<f64>> {
    const STRETCH_POINT: T;
    const SQUISH_POINT: T;

    fn eval<M: MathsCore>(point: T, perm: &PermTable, wrap: f64) -> f64;
    fn extrapolate<M: MathsCore>(grid: T, delta: T, perm: &PermTable, wrap: f64) -> f64;
}

fn generate_perm_array(seed: i64) -> PermTable {
    let mut perm: PermTable = [0; PSIZE as usize];
    let mut source = [0; PSIZE as usize];

    for i in 0..PSIZE {
        source[i as usize] = i;
    }

    let seed: i128 = (i128::from(seed) * 6_364_136_223_846_793_005) + 1_442_695_040_888_963_407;
    for i in (0..PSIZE).rev() {
        let mut r = ((seed + 31) % (i128::from(i) + 1)) as i64;
        if r < 0 {
            r += i + 1;
        }
        perm[i as usize] = source[r as usize];
        source[r as usize] = source[i as usize];
    }

    perm
}
