//! The Mk node update with aarch64 NEON intrinsics.
//!
//! The compiler already vectorizes the portable loops in `mod.rs`, one loop
//! at a time. This kernel takes eight patterns at a time through a whole node
//! update instead, with the weighted sums and the running maximum in
//! registers: the sums are not written and reread for every state, and
//! rescaling does not reread the results to find each pattern's maximum. Per
//! pattern it performs the same floating-point operations in the same order
//! as `LikelihoodCore::apply_mk` and `rescale`, so every result that is not
//! NaN is identical.
//!
//! The module is compiled only when the target enables NEON, as every
//! aarch64 target does by default; that is what the intrinsics require.
//! Loads and stores also need a pointer to two `f64` values, which `load`
//! and `store` take from a two-element array.

use core::arch::aarch64::{
    float64x2_t, vaddq_f64, vandq_u64, vbslq_u64, vcgtq_f64, vcgtq_u64, vcltq_f64, vdupq_n_f64,
    vdupq_n_s64, vdupq_n_u64, vld1q_f64, vmaxnmq_f64, vmulq_f64, vreinterpretq_f64_u64,
    vreinterpretq_s64_u64, vreinterpretq_u64_f64, vshlq_n_u64, vshrq_n_u64, vst1q_f64, vst1q_s64,
    vsubq_s64, vsubq_u64,
};

use crate::layout::{LANES, Layout};

/// Patterns per iteration: four registers of two lanes.
const BLOCK: usize = 8;
const _: () = assert!(LANES % BLOCK == 0, "the stride must hold whole blocks");

/// One child of the node being updated.
pub struct Child<'a> {
    pub partials: &'a [f64],
    /// The distance, `e` and `1 − e` of each rate category.
    pub transition: &'a [f64],
}

#[inline(always)]
fn load(x: &[f64]) -> float64x2_t {
    let x: &[f64; 2] = x.try_into().expect("two lanes");
    // SAFETY: NEON is enabled, and `vld1q_f64` reads the two values of `x`.
    unsafe { vld1q_f64(x.as_ptr()) }
}

#[inline(always)]
fn store(out: &mut [f64], value: float64x2_t) {
    let out: &mut [f64; 2] = out.try_into().expect("two lanes");
    // SAFETY: NEON is enabled, and `vst1q_f64` writes the two values of `out`.
    unsafe { vst1q_f64(out.as_mut_ptr(), value) }
}

#[inline(always)]
fn splat(a: f64) -> float64x2_t {
    // SAFETY: NEON is enabled.
    unsafe { vdupq_n_f64(a) }
}

#[inline(always)]
fn add(a: float64x2_t, b: float64x2_t) -> float64x2_t {
    // SAFETY: NEON is enabled.
    unsafe { vaddq_f64(a, b) }
}

#[inline(always)]
fn mul(a: float64x2_t, b: float64x2_t) -> float64x2_t {
    // SAFETY: NEON is enabled.
    unsafe { vmulq_f64(a, b) }
}

/// The NaN-ignoring maximum. It agrees with `f64::max` except when an operand
/// is a signalling NaN, which a product, a stored maximum or zero never is.
#[inline(always)]
fn max(a: float64x2_t, b: float64x2_t) -> float64x2_t {
    // SAFETY: NEON is enabled.
    unsafe { vmaxnmq_f64(a, b) }
}

#[inline(always)]
fn load_block(x: &[f64]) -> [float64x2_t; 4] {
    let x: &[f64; BLOCK] = x.try_into().expect("a block");
    [
        load(&x[0..2]),
        load(&x[2..4]),
        load(&x[4..6]),
        load(&x[6..8]),
    ]
}

#[inline(always)]
fn store_block(out: &mut [f64], values: [float64x2_t; 4]) {
    let out: &mut [f64; BLOCK] = out.try_into().expect("a block");
    for (out, value) in out.chunks_exact_mut(2).zip(values) {
        store(out, value);
    }
}

/// The same exponent extraction as `scaling::factors`, two patterns at a time.
pub(super) fn scale_factors(maxima: &mut [f64], exponents: &mut [i64]) {
    assert_eq!(maxima.len(), exponents.len());
    assert_eq!(maxima.len() % 2, 0);
    for (m, e) in maxima
        .chunks_exact_mut(2)
        .zip(exponents.chunks_exact_mut(2))
    {
        let maximum = load(m);
        // SAFETY: NEON is enabled. Both chunks contain two elements; the
        // exponent store writes exactly those two i64 values.
        unsafe {
            let valid = vandq_u64(
                vcgtq_f64(maximum, vdupq_n_f64(0.0)),
                vcltq_f64(maximum, vdupq_n_f64(f64::INFINITY)),
            );
            let raw = vshrq_n_u64::<52>(vreinterpretq_u64_f64(maximum));
            let capped = vbslq_u64(vcgtq_u64(raw, vdupq_n_u64(2045)), vdupq_n_u64(2045), raw);
            let biased = vbslq_u64(valid, capped, vdupq_n_u64(1023));
            vst1q_s64(
                e.as_mut_ptr(),
                vsubq_s64(vreinterpretq_s64_u64(biased), vdupq_n_s64(1023)),
            );
            let reciprocal = vshlq_n_u64::<52>(vsubq_u64(vdupq_n_u64(2046), biased));
            store(m, vreinterpretq_f64_u64(reciprocal));
        }
    }
}

/// For every rate category and state,
/// `dst = (e₁·L₁ + (1 − e₁)·Σⱼ πⱼ·L₁ⱼ)·(e₂·L₂ + (1 − e₂)·Σⱼ πⱼ·L₂ⱼ)`.
/// With `maxima`, also leaves there each pattern's largest result over states
/// and categories, as `rescale` computes it.
pub fn mk(
    l: &Layout,
    frequencies: &[f64],
    [first, second]: [Child<'_>; 2],
    dst: &mut [f64],
    mut maxima: Option<&mut [f64]>,
) {
    if let Some(m) = maxima.as_deref_mut() {
        m.fill(0.0);
    }
    for c in 0..l.categories {
        let (stay1, change1) = (first.transition[3 * c + 1], first.transition[3 * c + 2]);
        let (stay2, change2) = (second.transition[3 * c + 1], second.transition[3 * c + 2]);
        let (stay1, stay2) = (splat(stay1), splat(stay2));
        for block in (0..l.stride).step_by(BLOCK) {
            let mut sum1 = [splat(0.0); 4];
            let mut sum2 = [splat(0.0); 4];
            for (j, &f) in frequencies.iter().enumerate() {
                let o = l.offset(c, j) + block;
                let (w1, w2) = (splat(change1 * f), splat(change2 * f));
                let x1 = load_block(&first.partials[o..o + BLOCK]);
                let x2 = load_block(&second.partials[o..o + BLOCK]);
                for r in 0..4 {
                    sum1[r] = add(sum1[r], mul(w1, x1[r]));
                    sum2[r] = add(sum2[r], mul(w2, x2[r]));
                }
            }
            let mut largest = match maxima.as_deref() {
                Some(m) => load_block(&m[block..block + BLOCK]),
                None => [splat(0.0); 4],
            };
            for i in 0..l.states {
                let o = l.offset(c, i) + block;
                let x1 = load_block(&first.partials[o..o + BLOCK]);
                let x2 = load_block(&second.partials[o..o + BLOCK]);
                let out: [float64x2_t; 4] = core::array::from_fn(|r| {
                    mul(
                        add(mul(stay1, x1[r]), sum1[r]),
                        add(mul(stay2, x2[r]), sum2[r]),
                    )
                });
                for r in 0..4 {
                    largest[r] = max(largest[r], out[r]);
                }
                store_block(&mut dst[o..o + BLOCK], out);
            }
            if let Some(m) = maxima.as_deref_mut() {
                store_block(&mut m[block..block + BLOCK], largest);
            }
        }
    }
}
