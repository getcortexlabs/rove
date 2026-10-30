use super::Backend;

/// Replace maxima with finite, normal multipliers and record their inverse
/// binary exponents. For an ordinary maximum, e = floor(log2(maximum)).
/// Clamp e to [-1023, 1022] so 2^-e is representable and normal even at the
/// ends of f64's range. Subnormal maxima need no reciprocal or logarithm.
pub(super) fn factors(maxima: &mut [f64], exponents: &mut [i64], backend: Backend) {
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    if backend == Backend::Neon {
        super::neon::scale_factors(maxima, exponents);
        return;
    }
    let _ = backend;
    for (maximum, exponent) in maxima.iter_mut().zip(exponents) {
        let biased = if *maximum > 0.0 && maximum.is_finite() {
            (maximum.to_bits() >> 52).min(2045)
        } else {
            1023
        };
        *exponent = biased as i64 - 1023;
        *maximum = f64::from_bits((2046 - biased) << 52);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_exponent_and_its_boundaries_scale_without_overflow() {
        let mut values = vec![0.0, -0.0, f64::INFINITY, f64::NAN];
        for exponent in 0..2047 {
            for mantissa in [0, 1, (1 << 51) - 1, 1 << 51, (1 << 52) - 1] {
                values.push(f64::from_bits((exponent << 52) | mantissa));
            }
        }
        values.resize(values.len().next_multiple_of(8), 0.0);
        let mut expected = None;
        for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
            let mut multipliers = values.clone();
            let mut exponents = vec![0; values.len()];
            factors(&mut multipliers, &mut exponents, backend);
            for ((&value, &multiplier), &exponent) in
                values.iter().zip(&multipliers).zip(&exponents)
            {
                assert!(multiplier.is_normal() && multiplier > 0.0);
                if value > 0.0 && value.is_finite() {
                    let scaled = value * multiplier;
                    assert!(scaled >= 2f64.powi(-51) && scaled < 4.0);
                    assert_eq!((scaled / multiplier).to_bits(), value.to_bits());
                    assert!(
                        (scaled.ln() + exponent as f64 * std::f64::consts::LN_2 - value.ln()).abs()
                            < 2e-13
                    );
                } else {
                    assert_eq!((multiplier, exponent), (1.0, 0));
                }
            }
            let actual = (multipliers, exponents);
            if let Some(expected) = &expected {
                assert_eq!(&actual, expected);
            } else {
                expected = Some(actual);
            }
        }
    }
}
