# Estimating the FBD process

This fixture defines the `joint_sampling_v1` model used when at least one FBD
parameter is estimated. It is a different conditioning choice from a tree prior
normalized separately for every parameter value. It does not add sampled
ancestors or marginal-likelihood estimation.

## Target and conditioning

Let T be a dated, labeled tree, θ = (λ, μ, ψ, ρ, o) the process and origin,
c the character clock/rate parameters, D the characters, A the specimen age
evidence, and E the observed specimen count and imposed terminal/clade constraints.
The implemented target, up to a single evidence denominator, is

```
p(T, θ, c | D, A, E) ∝
    L(D | T, c) f_FBD(T | θ) p(θ) p(c) L(A | T) I(E, T).
```

Here f_FBD is the origin-conditioned sampling density, with removal probability
r = 0, without conditioning on survival or at least one present-day sample.
Nonzero-duration terminal fossil branches are required. Terminal status and
monophyly are imposed assumptions; the character data did not establish them.
The underlying process allows sampled ancestors, but E excludes their histories.
The observed specimen count and ages remain sampling information. With fixed
ages, the corresponding density is evaluated at those ages; interval evidence
has a uniform factor on its stated support.

The integral `Z_E(θ) = ∫ f_FBD(T | θ) L(A | T) I(E,T) dT` generally depends on
θ. This model **retains** that mass when estimating θ. It is not the alternative
`p(θ) f_FBD(T | θ) L(A | T) I(E,T) / Z_E(θ)`, which normalizes the restricted tree
space separately for each θ. Simply assuming Z_E constant would be incorrect.
The overall denominator integrates both tree and process. `--sample-from-prior`
removes L(D | T,c), but still conditions on A and E; its process marginals need
not equal the nominal hyperpriors.

For all-fixed θ, both conditioning choices yield the same normalized tree
posterior because Z_E is constant in the sampled state. Old configurations keep
that behavior (`fixed_process_restriction_v1`). Estimated configurations require
an explicit `process_model: "joint_sampling_v1"` marker and `process_priors`.
This preview does not calculate either model's evidence.

The pinned convention comes from [BEAST 2 SA 2.1.1's source](https://github.com/CompEvol/sampled-ancestors/blob/v2.1.1/src/sa/evolution/speciation/SABirthDeathModel.java)
and its [input definitions](https://www.beast2.org/xml/sa.evolution.speciation.SABirthDeathModel.html).
The [RevBayes specimen tutorial](https://revbayes.github.io/tutorials/fbd/fbd_specimen)
discusses using specimen age evidence as a likelihood rather than treating a
specimen's interval as a species occurrence range. Rove's explicit joint target
above is the definition being validated, not a claim of equivalence to every
BEAST or RevBayes dating setup.

## Priors and proposals

| Quantity | Optional prior | Proposal and log Hastings ratio |
| --- | --- | --- |
| λ, μ, ψ | Exponential with specified mean m: `−ln(m) − x/m` | `x' = x f`, f uniform on [s, 1/s], ratio `−ln(f)` |
| ρ | Beta(α,β), including `ln Γ(α+β) − ln Γ(α) − ln Γ(β)` | Symmetric uniform step on logit(ρ); ratio `ln[ρ'(1−ρ')] − ln[ρ(1−ρ)]` |
| o | Exponential on o − a*, a* = largest allowed tip age | Scale o − a* as above; ratio `−ln(f)` |

These are the only process priors currently supported. Estimated rates are
strictly positive; fixed death can be zero. Estimated ρ lies strictly in (0,1),
with beta shapes in [0.001,1000]. A missing prior fixes that parameter. Each
parameter has its own optional move, initially weighted 3. All five values are
part of accepted/rejected state restoration and exact checkpoint continuation.

Origin also has to exceed the current root and a*. The shifted exponential has
normalizer 1/m, with no root-dependent renormalization. Its offset is explicitly
chosen from the supplied age evidence; it is part of the model configuration.
The clock has its existing optional exponential prior, independent of these
choices. All-fixed process traces retain their previous columns.

## Independent checks

`tests/fbd_estimation.rs` tests distributions, not just density agreement:

- For two extant tips, μ=0, ρ=1, root height h and origin o,
  `f_FBD = 2 λ exp[−(λ+ψ)(o+h)]`. At a fixed tree, exponential priors give
  λ ~ Gamma(2, rate o+h+1/m), ψ ~ Exp(rate o+h+1/m), and
  o ~ h + Exp(rate λ+ψ+1/m). Each sampled conditional mean is checked.
- Integrating λ with prior mean 1, fixed o=4 and ψ=0.2 gives the independent
  root-height target proportional to `exp[−0.2(4+h)] / (5+h)^2` on (0,4).
  Numerical integration gives reference joint means for h and λ.
- For an extant tip and a fossil at age a, an independent backward ODE computes
  `u' = μ − (λ+μ+ψ)u + λu²`, `q' = (2λu − λ−μ−ψ)q`, starting with
  u(0)=1−ρ, q(0)=1. The tree density is
  `2 λ ψ ρ q(o) q(h) u(a)/q(a)`. RK4 integration and one-dimensional quadrature
  supply separate reference means for μ and ρ, including the proper priors.
- Eight fixed states compare the tree factor, summed parameter priors and total
  joint density against BEAST.base 2.7.8 / SA 2.1.1, within 2e-12 absolute log
  density. Priors are exponential means 1.1, 0.6, 0.3 for λ, μ, ψ, Beta(2,3)
  for ρ, and mean excess 2 above fossil age 0.7 for origin. The clock is fixed.
- Mixed fixed/estimated CLI runs verify trace values and fixed diagnostics,
  reject invalid support/model specifications, and generate posterior predictions.
  Rust and native tests verify rejection restoration and exact resumed samples.
  The existing allocation test exercises all five new moves with a relaxed clock.

These tests do not establish posterior equivalence on an empirical dataset.
The [four-tip workload report](../../../benchmarks/fbd-estimation/2026-10-30/README.md)
records independent-chain diagnostics, parameter correlations and measured cost.

## Reproduce

The four-tip, eight-character matrix is reused from [the dating fixture](../dating).
It has four compressed patterns. `dating.json` adds all five priors to that
fixture; the numerical oracle above uses its separate two-tip states.

```sh
cargo test --release --test fbd_estimation --test dating --test allocations
cargo test --release --test cli estimated_processes_preserve_live_values_and_fixed_diagnostics
cargo run --release --bin rove -- sample fixtures/validation/dating/matrix.nex \
  --dating fixtures/validation/fbd-estimation/dating.json \
  --output runs/fbd-estimation --model reference \
  --seed 42 --steps 1000000 --sample-every 100 --chains 4
```

To regenerate the numerical reference, set `BEAST_BASE_JAR` to the pinned
BEAST.base.jar and `SA_JAR` to SA.v2.1.1.jar. Their hashes and Java version are
recorded in `provenance.json`. From the repository root:

```sh
mkdir -p .local/fbd-estimation-oracle
javac -cp "$BEAST_BASE_JAR:$SA_JAR" -d .local/fbd-estimation-oracle \
  tools/beast/FbdEstimationStates.java
java -cp ".local/fbd-estimation-oracle:$BEAST_BASE_JAR:$SA_JAR" \
  FbdEstimationStates > .local/fbd-estimation-oracle/beast.tsv
cmp .local/fbd-estimation-oracle/beast.tsv \
  fixtures/validation/fbd-estimation/beast-sa-2.1.1.tsv
```
