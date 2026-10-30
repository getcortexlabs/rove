---
description: "Date fossil tips with fixed ages or age intervals, an explicit time scale and a fossilized birth–death prior."
---

# Fossil tip dating

`rove sample --dating settings.json` adds fossil ages and a time scale to a morphological analysis. It works with strict or relaxed clocks, character partitions, ascertainment conditioning, independent chains and checkpoints. Configuration is currently through the Rust CLI; the Mac setup screen has no dating controls. BEAST and Java are not required.

## Ages and units

Age is measured **backwards from the present**: zero is an extant taxon, 12 means 12 units before present. In the example, `time_unit: "Ma"` means millions of years. All ages, node heights, origin and branch durations must use this same unit; Rove records the label but does not convert units.

The clock rate is expected character changes per unit time. A branch's expected changes are its duration × the global clock rate × its relative branch rate. The relaxed clock adds relative branch rates with distribution mean one; gamma categories additionally multiply the character rate. Without dating, the existing presets still use a unit clock and zero-age tips.

Every taxon must have an explicit age. A number fixes it; `[minimum, maximum]` estimates it within that interval. Positive intervals describe uncertainty in a **specimen's age**, not a species' first-to-last occurrence range. That distinction follows the [RevBayes specimen tutorial](https://revbayes.github.io/tutorials/fbd/fbd_specimen). Intervals cannot span the present; `[12,12]` is fixed. Names must match the parsed NEXUS labels, where an unquoted underscore becomes a space.

## Try the constructed example

```sh
cargo run --release --bin rove -- sample fixtures/validation/dating/matrix.nex \
  --dating fixtures/validation/dating/dating.json \
  --output runs/dated-example --seed 25 --steps 100000 --sample-every 100
```

The example is a software check, not a biological analysis or a convergence guarantee. Its JSON contains:

```json
{
  "time_unit": "Ma",
  "birth_rate": 0.08,
  "death_rate": 0.03,
  "fossil_sampling_rate": 0.02,
  "present_sampling": 0.7,
  "origin": 50,
  "clock_rate": 0.01,
  "clock_prior_mean": 0.01,
  "ages": {
    "Living A": 0,
    "Fossil B": [5, 9],
    "Living C": 0,
    "Fossil D": 12
  }
}
```

Birth, death and fossil sampling are per-lineage rates per time unit; present_sampling is a probability, not a rate. Origin is the age at which the stem lineage began, older than the root and every allowed tip age. These five process settings remain fixed. Choose them for the scientific question and examine sensitivity to their values; the example values are not defaults.

With clock_prior_mean, clock_rate supplies the initial value and the clock is estimated with an exponential prior of that mean. Omit clock_prior_mean to fix clock_rate. Use `--model reference` for gamma categories and the relaxed clock, or `--sample-from-prior` to inspect the joint prior without character likelihoods.

## What the prior assumes

The FBD process includes birth, extinction, fossil sampling and incomplete present-day sampling. This implementation restricts trees to **terminal fossils**, without sampled ancestors. Sampling a fossil does not force the lineage to die: the model uses removal probability zero. Origin and process parameters stay fixed so the normalization of the restricted tree space is constant throughout sampling.

The reported FBD log density matches the fixed-origin, unconditioned-on-survival convention of [BEAST 2 SA](https://www.beast2.org/xml/sa.evolution.speciation.SABirthDeathModel.html) at the validated states. It is an **unnormalized restricted density**, unsuitable for marginal likelihoods or Bayes factors. This version does not estimate the process parameters or origin, sample ancestors, or model species occurrence ranges. An age interval constrains the analysis; recovering that interval is not independent evidence for a fossil's age.

## Outputs and checks

The trace contains fossilized_birth_death, clock_rate, dating_parameter_prior and tip_age_1, tip_age_2, etc., in matrix order. Existing gamma and relative-clock columns remain when enabled. Fixed parameters have unavailable ESS, as expected; chain comparisons mark them fixed_by_model and exclude only those verified fixed values from automatic stopping checks. A sampled parameter that becomes constant still blocks stopping.

`run.json` stores the complete dating settings and their fingerprint. Checkpoints embed the settings, so resuming does not require the original JSON. Trees retain explicit heights and branch durations; relative rate annotations do not include the global clock, which is saved in the trace. `rove summarize`, `inspect-tree`, `diagnose`, `compare` and `predict` accept these outputs. Summaries preserve the time-unit label when the run record is available. A standalone tree file does not identify the physical time unit by itself.

Validation compares eight fixed densities against BEAST 2.7.8 / SA 2.1.1, checks small-tree prior distributions against an analytic limit and independent ODE quadrature, and exercises likelihood restoration, time-unit invariance, restart, exports and allocation-free proposals. These checks establish implemented behavior; they do not establish convergence or performance on an empirical fossil dataset. The [validation fixture](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/dating) records the formulas, source, versions and reproduction commands.
