use super::{Error, Result, quantile, sum};

/// Rank-normalized split R-hat, including folded ranks (Vehtari et al., 2021).
/// A common constant has no diagnostic; chains stuck at different values have
/// infinite R-hat. Neither case is evidence of convergence.
pub fn rhat(chains: &[&[f64]]) -> Result<Option<f64>> {
    if chains.len() < 2
        || chains[0].len() < 4
        || chains
            .iter()
            .any(|c| c.len() != chains[0].len() || c.iter().any(|x| !x.is_finite()))
    {
        return Err(Error(
            "R-hat requires at least two equal-length finite chains with four samples".into(),
        ));
    }
    if chains
        .iter()
        .flat_map(|c| c.iter())
        .all(|&x| x == chains[0][0])
    {
        return Ok(None);
    }
    let half = chains[0].len() / 2;
    let split: Vec<_> = chains
        .iter()
        .flat_map(|c| c[..half].iter().chain(&c[c.len() - half..]))
        .copied()
        .collect();
    let bulk = variance_ratio(&rank_normalize(&split), half);
    let mut sorted = split.clone();
    sorted.sort_unstable_by(f64::total_cmp);
    let median = quantile(&sorted, 0.5);
    let mut folded: Vec<_> = split.iter().map(|x| (x - median).abs()).collect();
    if folded.iter().any(|x| !x.is_finite()) {
        // A common power-of-two scaling preserves ordering when a finite
        // extreme draw minus the median would overflow.
        folded = split
            .iter()
            .map(|x| (x * 0.5 - median * 0.5).abs())
            .collect();
    }
    if folded.iter().all(|x| *x == folded[0]) {
        return Ok(Some(bulk));
    }
    Ok(Some(
        bulk.max(variance_ratio(&rank_normalize(&folded), half)),
    ))
}

fn variance_ratio(values: &[f64], width: usize) -> f64 {
    let means: Vec<_> = values
        .chunks_exact(width)
        .map(|c| sum(c.iter().copied()) / width as f64)
        .collect();
    let within = sum(values
        .chunks_exact(width)
        .zip(&means)
        .map(|(c, mean)| sum(c.iter().map(|x| (x - mean).powi(2))) / (width - 1) as f64))
        / means.len() as f64;
    if within == 0.0 {
        return f64::INFINITY;
    }
    let mean = sum(means.iter().copied()) / means.len() as f64;
    let between = sum(means.iter().map(|x| (x - mean).powi(2))) / (means.len() - 1) as f64;
    ((width - 1) as f64 / width as f64 + between / within).sqrt()
}

fn rank_normalize(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<_> = (0..values.len()).collect();
    order.sort_unstable_by(|&a, &b| values[a].total_cmp(&values[b]));
    let mut result = vec![0.0; values.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && values[order[end]] == values[order[start]] {
            end += 1;
        }
        let rank = (start + 1 + end) as f64 / 2.0;
        let score = normal_quantile((rank - 0.375) / (values.len() as f64 + 0.25));
        for &i in &order[start..end] {
            result[i] = score;
        }
        start = end;
    }
    result
}

// Acklam's inverse-normal rational approximation, relative error < 1.2e-9.
// Ranks always give probabilities strictly inside (0, 1).
fn normal_quantile(p: f64) -> f64 {
    let a = [
        -3.969683028665376e1,
        2.209460984245205e2,
        -2.759285104469687e2,
        1.383_577_518_672_69e2,
        -3.066479806614716e1,
        2.506628277459239,
    ];
    let b = [
        -5.447609879822406e1,
        1.615858368580409e2,
        -1.556989798598866e2,
        6.680131188771972e1,
        -1.328068155288572e1,
    ];
    let c = [
        -7.784894002430293e-3,
        -3.223964580411365e-1,
        -2.400758277161838,
        -2.549732539343734,
        4.374664141464968,
        2.938163982698783,
    ];
    let d = [
        7.784695709041462e-3,
        3.224671290700398e-1,
        2.445134137142996,
        3.754408661907416,
    ];
    let poly = |coefficients: &[f64], x: f64| coefficients.iter().fold(0.0, |y, a| y * x + a);
    if !(0.02425..=0.97575).contains(&p) {
        let q = (-2.0 * p.min(1.0 - p).ln()).sqrt();
        let x = poly(&c, q) / (poly(&d, q) * q + 1.0);
        if p < 0.5 { x } else { -x }
    } else {
        let q = p - 0.5;
        let r = q * q;
        poly(&a, r) * q / (poly(&b, r) * r + 1.0)
    }
}
