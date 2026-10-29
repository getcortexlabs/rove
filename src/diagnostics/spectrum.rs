use super::{Error, Result};
use std::f64::consts::TAU;

#[derive(Clone, Copy, Default)]
struct Complex {
    re: f64,
    im: f64,
}
impl Complex {
    fn multiply(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
}

fn fft(values: &mut [Complex], inverse: bool, cancel: &dyn Fn() -> bool) -> Result<()> {
    let n = values.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            values.swap(i, j);
        }
    }
    let mut width = 2;
    while width <= n {
        if cancel() {
            return Err(Error("trace diagnostics cancelled".into()));
        }
        let angle = if inverse { TAU } else { -TAU } / width as f64;
        let step = Complex {
            re: angle.cos(),
            im: angle.sin(),
        };
        for block in values.chunks_exact_mut(width) {
            let mut phase = Complex { re: 1.0, im: 0.0 };
            for i in 0..width / 2 {
                let a = block[i];
                let b = block[i + width / 2].multiply(phase);
                block[i] = Complex {
                    re: a.re + b.re,
                    im: a.im + b.im,
                };
                block[i + width / 2] = Complex {
                    re: a.re - b.re,
                    im: a.im - b.im,
                };
                phase = phase.multiply(step);
            }
        }
        width *= 2;
    }
    if inverse {
        for value in values {
            value.re /= n as f64;
            value.im /= n as f64;
        }
    }
    Ok(())
}

pub(super) fn autocovariance(
    values: &[f64],
    lags: usize,
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<f64>> {
    // Zero padding prevents circular correlation from wrapping the trace's end
    // around to its beginning. Normalization uses N-lag, matching the reference.
    let mut spectrum = vec![Complex::default(); (2 * values.len()).next_power_of_two()];
    for (point, &value) in spectrum.iter_mut().zip(values) {
        point.re = value;
    }
    fft(&mut spectrum, false, cancel)?;
    for point in &mut spectrum {
        *point = Complex {
            re: point.re * point.re + point.im * point.im,
            im: 0.0,
        };
    }
    fft(&mut spectrum, true, cancel)?;
    Ok((0..lags)
        .map(|lag| spectrum[lag].re / (values.len() - lag) as f64)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fft_matches_direct_autocovariance_including_non_power_of_two_lengths() {
        for n in [4, 7, 16, 31, 100, 513, 2001] {
            let values: Vec<_> = (0..n)
                .map(|i| (i as f64 * 1.73).sin() + (i as f64 * 0.13).cos())
                .collect();
            let lags = n.min(2000);
            let actual = autocovariance(&values, lags, &|| false).unwrap();
            for lag in 0..lags {
                let expected = values[..n - lag]
                    .iter()
                    .zip(&values[lag..])
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
                    / (n - lag) as f64;
                assert!(
                    (actual[lag] - expected).abs() < 2e-10,
                    "n={n}, lag={lag}: {} != {expected}",
                    actual[lag]
                );
            }
        }
    }
}
