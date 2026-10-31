---
description: "Date fossil tips with fixed ages or age intervals, an explicit time scale and a fossilized birth–death prior."
---

# Fossil tip dating

`rove sample --dating settings.json` adds fossil ages and a time scale to a morphological analysis. It works with strict or relaxed clocks, character partitions, ascertainment conditioning, independent chains and checkpoints. Configure it in the native Mac app or through the Rust CLI. BEAST and Java are not required.

## In the Mac app

1. Import the matrix, then open **Model** and enable **Date fossils** under **A place in time**.
2. Choose the common time unit and open **Edit tip ages**. Every terminal starts as present-day (zero). Search for each fossil and choose a fixed age or a younger–older interval, then **Apply ages**. This editor uses distinct bounds for intervals; use **Fixed age** when they are equal.
3. For each process parameter, choose **Keep fixed** or **Estimate**. Enter a fixed value, or an initial value and prior. Rates use an exponential prior mean; present-day sampling uses beta shapes α and β; origin uses a mean excess above the oldest allowed tip age. Fields start empty. Example values below are software checks, not biological recommendations.
4. Set the global clock to **Estimate** (initial rate and exponential prior mean) or **Keep fixed** (rate only). The evolutionary clock cards still choose strict versus relaxed branch rates.
5. Check the full setup in **Review**, then start the analysis. History retains the age assignments, unit, process settings and clock mode; saved setups and paused runs preserve them too.

Decimal dots and commas are accepted, as is scientific notation. Grouping separators are not accepted. Changing the unit label does not rescale any value. Disabling dating restores the undated model while keeping the entered values available if you enable it again; importing another matrix starts a fresh setup.

Annotated trees show the recorded time unit on their scale. Chain comparisons label verified fixed parameters as **Fixed**, because ESS and R-hat do not apply to values that were never sampled.

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

Birth, death and fossil sampling are per-lineage rates per time unit; present_sampling is a probability, not a rate. Origin is the age at which the stem lineage began, older than the root and every allowed tip age. In this example, these five process settings remain fixed. Optional priors below enable estimation independently. Choose them for the scientific question and examine sensitivity to their values; the example values are not defaults.

With clock_prior_mean, clock_rate supplies the initial value and the clock is estimated with an exponential prior of that mean. Omit clock_prior_mean to fix clock_rate. Use `--model reference` for gamma categories and the relaxed clock, or `--sample-from-prior` to omit character likelihoods. Sample counts, age evidence and imposed constraints still inform estimated process parameters.

## Fixed or estimated process parameters

Add these two fields to the dating JSON to estimate all five process parameters. This is a fragment, not a complete configuration. A ready-to-run version is at `fixtures/validation/fbd-estimation/dating.json`:

```json
{
  "process_model": "joint_sampling_v1",
  "process_priors": {
    "birth_rate_mean": 0.08,
    "death_rate_mean": 0.03,
    "fossil_sampling_rate_mean": 0.02,
    "present_sampling_beta": [2, 2],
    "origin_excess_mean": 30
  }
}
```

Omit an individual prior to keep its parameter fixed; retain at least one prior in `process_priors`. Both the explicit model marker and the prior object are required. The corresponding top-level values become initial values for estimated parameters. Existing files without either field keep their fixed-process behavior.

Exponential means must be positive and finite. Estimated rates start strictly above zero; fixed death may be zero. Beta shapes must be in [0.001, 1000], with initial sampling probability strictly between zero and one. The beta prior mean is α / (α + β).

`origin_excess_mean` is the mean of an exponential distribution on **origin minus the largest allowed tip age**. In this example that bound is 12 Ma, so a mean excess of 30 Ma gives a nominal origin-prior mean of 42 Ma. Origin must also exceed the current root. That joint support is enforced without renormalizing the origin prior at each root height.

Rates, time, origin and the clock can trade off against each other. Estimating everything can produce broad, correlated posteriors and slower exploration. Examine traces, independent-chain diagnostics and sensitivity to scientifically justified priors; a high ESS alone does not show that the data identify every parameter.

## What the prior assumes

The FBD process includes birth, extinction, fossil sampling and incomplete present-day sampling. Trees are restricted to **terminal fossils**, without sampled ancestors. This is an imposed constraint, not evidence recovered by Rove. Sampling does not force a lineage to die: removal probability is zero.

With all process parameters fixed, the restricted tree density has a constant omitted normalizer. Estimation explicitly selects `joint_sampling_v1`: the observed specimen count, age evidence and imposed terminal/clade restrictions inform the process as well as the tree. Rove multiplies the FBD sampling density by proper parameter priors and the observation factors. The restricted tree mass depends on the parameters and is retained as information; it is **not** assumed to cancel, or divided out separately at each parameter value. This differs from a tree prior conditioned and normalized separately for each process.

The underlying FBD density uses origin conditioning, without survival or present-sampling conditioning, matching [BEAST 2 SA](https://www.beast2.org/xml/sa.evolution.speciation.SABirthDeathModel.html) at validated states. The [model derivation and validation](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/fbd-estimation) specify this joint target. Marginal likelihoods, sampled ancestors and species occurrence ranges remain unsupported. Imposed age bounds or clades are not independent recovery from the character data.

## Outputs and checks

The trace contains fossilized_birth_death, clock_rate, dating_parameter_prior and tip_age_1, tip_age_2, etc., in matrix order. Existing gamma and relative-clock columns remain when enabled. Fixed parameters have unavailable ESS, as expected; chain comparisons mark them fixed_by_model and exclude only those verified fixed values from automatic stopping checks. A sampled parameter that becomes constant still blocks stopping.

`run.json` stores the complete dating settings and their fingerprint. Checkpoints embed the settings, so resuming does not require the original JSON. Trees retain explicit heights and branch durations; relative rate annotations do not include the global clock, which is saved in the trace. `rove summarize`, `inspect-tree`, `diagnose`, `compare` and `predict` accept these outputs. Summaries preserve the time-unit label when the run record is available. A standalone tree file does not identify the physical time unit by itself.

Validation compares eight fixed densities against BEAST 2.7.8 / SA 2.1.1, checks small-tree prior distributions against an analytic limit and independent ODE quadrature, and exercises likelihood restoration, time-unit invariance, restart, exports and allocation-free proposals. These checks establish implemented behavior; they do not establish convergence or performance on an empirical fossil dataset. The [validation fixture](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/dating) records the formulas, source, versions and reproduction commands.

When at least one process parameter is estimated, the trace additionally records `fbd_birth_rate`, `fbd_death_rate`, `fbd_fossil_sampling_rate`, `fbd_present_sampling` and `fbd_origin`. Fixed members are recorded too and verified as fixed by chain comparisons. `run.json` records `dating_density_convention` and the full priors. Native saved setups use format 4 for process estimation, format 3 for fixed-process dating and format 2 for undated analyses; older apps reject unsupported formats. Checkpoint continuation requires the same engine executable.

Process-estimation checks add eight complete prior states against BEAST 2.7.8 / SA 2.1.1, analytic parameter conditionals, independent joint integration, mixed fixed/estimated runs and exact restart tests. The [constructed workload report](https://github.com/getcortexlabs/rove/tree/main/benchmarks/fbd-estimation/2026-10-30) records runtime, ESS, R-hat, clade differences and correlations. It does not establish accuracy, identifiability or performance on empirical fossil data.

The [gamma-frequency study](./reference-model#gamma-proposal-frequency) identifies infrequent gamma updates as a sampling limitation in this constructed example. A tested CLI override improves gamma ESS, but does not meet every efficiency criterion for a new default. The native app retains the existing schedule.
