use crate::layout::Layout;

pub(super) struct Child<'a> {
    pub partials: &'a [f64],
    /// Distance, exp(-d/beta), and 1 - exp(-d/beta), per rate category.
    pub transition: &'a [f64],
}

/// The portable Mk operation order, shared by dense and packed patterns.
pub(super) fn prune(
    l: Layout,
    frequencies: &[f64],
    [first, second]: [Child<'_>; 2],
    dst: &mut [f64],
    acc1: &mut [f64],
    acc2: &mut [f64],
) {
    let stride = l.stride;
    for c in 0..l.categories {
        let (stay1, change1) = (first.transition[3 * c + 1], first.transition[3 * c + 2]);
        let (stay2, change2) = (second.transition[3 * c + 1], second.transition[3 * c + 2]);
        acc1.fill(0.0);
        acc2.fill(0.0);
        for (j, &f) in frequencies.iter().enumerate() {
            let o = l.offset(c, j);
            let (a, b) = (change1 * f, change2 * f);
            for (acc, x) in acc1.iter_mut().zip(&first.partials[o..o + stride]) {
                *acc += a * x;
            }
            for (acc, x) in acc2.iter_mut().zip(&second.partials[o..o + stride]) {
                *acc += b * x;
            }
        }
        for i in 0..l.states {
            let o = l.offset(c, i);
            let children = first.partials[o..o + stride]
                .iter()
                .zip(&second.partials[o..o + stride]);
            let sums = acc1.iter().zip(&*acc2);
            for ((d, (x1, x2)), (s1, s2)) in dst[o..o + stride].iter_mut().zip(children).zip(sums) {
                *d = (stay1 * x1 + s1) * (stay2 * x2 + s2);
            }
        }
    }
}
