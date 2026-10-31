# Fossilized birth–death validation

The four-taxon matrix and JSON are constructed CLI examples, not empirical data.
The JSON's values are illustrative, not biological defaults. Underscores in
unquoted NEXUS labels are decoded to spaces; the JSON uses those decoded labels.

## Density convention

The engine evaluates the constant-rate FBD density of Gavryushkina et al. (2014),
with removal probability r=0, fixed origin, and neither survival nor present-day
sampling conditioning. Its implementation uses the same labeled-tree convention
as BEAST 2's SA package, including the `(n-1) log(2)` term. Binary trees have no
sampled ancestors. Fossil sampling does not imply removal of the lineage.

Let lambda, mu, psi, rho and o denote birth, death, fossil sampling, present-day
sampling and origin. Define

```
c = sqrt((lambda - mu - psi)^2 + 4*lambda*psi)
b = 1 + (2*lambda*rho - lambda + mu + psi)/c
Q(t) = exp(c*t) * (b + (2-b)*exp(-c*t))^2
p0(t) = probability of no sampled descendants over time t
```

For n tips, internal heights h, fossil ages a>0 and e extant samples:

```
log f = -log Q(o) + (n-1) log 2
      + sum_internal [log lambda - log Q(h)]
      + sum_fossils [log psi + log Q(a) + log p0(a)]
      + e log(4*rho).
```

The positive-root solution of the backward equation for p0 avoids subtraction
near the present. Q is evaluated in log space, without exp(c*t), so large origins
do not overflow. Every edge must have positive duration and the root must be
strictly younger than the origin. An age of exactly zero denotes an extant tip;
positive ages remain fossils even below BEAST SA's 5e-12 numerical cutoff.

Rove restricts this density to the supplied taxa, terminal fossils, age bounds
and monophyly constraints. Normalization is constant for tree, tip-age and clock
moves **because the process rates, rho and origin are fixed**. The logged value
is not the normalized restricted density and cannot be used as a marginal
likelihood. Estimating these fixed process parameters requires a separate
conditioning/normalization design. This is not an unrestricted SA analysis.

A fixed age is a constraint, not a zero-width uniform density. For a nonzero
interval [a,b], the age observation contributes -log(b-a) within its bounds.
The estimated global clock has Exponential(mean=clock_prior_mean), including
its normalizer; omitting that setting fixes the clock_rate. No other dating
parameter prior is introduced. These components occupy dating_parameter_prior,
separately from fossilized_birth_death and the existing gamma/relative-rate priors.

## Independent checks

`beast-sa-2.1.1.tsv` was exported by `tools/beast/FossilizedBirthDeathStates.java`
using **BEAST.base 2.7.8, SA 2.1.1 and Java 17**. Cases include incomplete extant
sampling, death greater than birth, equal birth/death rates, an entirely fossil
tree, near-present sampling and a time-unit change. Node order is A,B,C,(A,B),root;
C has age_a. All settings and heights are in the table. Removal probability is
zero; all three conditioning switches are false. The source and jar fingerprints
are recorded in `provenance.json`.

The tests use 1e-12 absolute log-density tolerance, except 1e-6 for the 1e-9-age
case, where SA's subtraction loses precision. They also check finite results for
an origin of 2000 time units, outside the reference implementation's exp range.
This is a fixed-state density comparison, not a posterior or speed comparison.

Independent sampling checks include:

- Two extant tips with mu=0 and rho=1: root height follows the truncated
  exponential distribution with rate lambda+psi, on (0,origin).
- Tip-age proposals sample a polynomial synthetic target, including intervals
  truncated by the current parent height.
- A fossil with uncertain age: joint age/root moments agree with numerical
  quadrature of the birth–death backward ODEs, integrated by RK4 rather than
  the production closed-form expression.
- Three tips with one fossil: topology probabilities agree with integrated
  prior masses under narrow exchange, wide exchange and subtree slide.
- Dated relaxed-clock chains preserve incremental likelihoods through accepted
  and rejected proposals, and checkpoints resume the exact sequence of states.
- The CLI exports dates, summarizes keep/CA heights, predicts matrices, compares
  independent chains and resumes without the original JSON. The allocation
  test checks 20,000 dated relaxed-clock proposals without heap allocations.
- Sampled trees carry one metadata block per node, `[&height=h]` or
  `[&height=h,rate=r]`. `tools/tree-summary/ReadTrees.java` reads a complete
  `trees.nexus` through BEAST.base 2.7.8's `NexusParser` and prints every
  node's height and rate. Freshly written strict, relaxed and estimated-age
  output must parse without preprocessing. Earlier relaxed-clock runs wrote
  `[&height=h][&rate=r]`, which that parser rejects; Rove reads both forms.

To regenerate the oracle, supply the pinned jars (validation only):

```sh
javac --release 17 -cp "$BEAST_BASE_JAR:$SA_JAR" -d /tmp/rove-fbd \
  tools/beast/FossilizedBirthDeathStates.java
java -cp "/tmp/rove-fbd:$BEAST_BASE_JAR:$SA_JAR" FossilizedBirthDeathStates \
  > /tmp/beast-sa-2.1.1.tsv
cargo test --test dating
```

To check the tree export against the same parser (the tree parser loads classes from BEAST's `launcher.jar` at run time):

```sh
javac --release 17 -cp "$BEAST_BASE_JAR" -d /tmp/rove-trees tools/tree-summary/ReadTrees.java
java -cp "/tmp/rove-trees:$BEAST_BASE_JAR:$BEAST_LAUNCHER_JAR" ReadTrees runs/dated-example/trees.nexus
```

References: [Gavryushkina et al. 2014](https://doi.org/10.1371/journal.pcbi.1003919),
[BEAST SA 2.1.1 source](https://github.com/CompEvol/sampled-ancestors/blob/v2.1.1/src/sa/evolution/speciation/SABirthDeathModel.java),
[RevBayes specimen age tutorial](https://revbayes.github.io/tutorials/fbd/fbd_specimen).
