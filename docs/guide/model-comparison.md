---
description: "Estimate log marginal likelihoods from power posteriors with stepping-stone and path sampling, and compare models through Bayes factors."
---

# Model comparison

`rove marginal` estimates the log marginal likelihood of a configured model: the probability of the matrix under the model with every tree and parameter integrated out. The difference between two models' log marginal likelihoods is the log Bayes factor. The estimate comes from power posteriors, the chain's target with the likelihood raised to an exponent between zero, which gives the prior, and one, which gives the posterior. The approach follows the [RevBayes model-selection tutorial](https://revbayes.github.io/tutorials/model_selection_bayes_factors/bf_intro) and the PathSampler of BEAST 2's MODEL_SELECTION package. No external software is required to run it.

This command is available in the Rust CLI. The Mac app does not expose it.

## Run an estimate

```sh
cargo run --release --bin rove -- marginal fixtures/reference/20x50/matrix.nex \
  --output runs/marginal-mkv --seed 1 --steps 1000000 --sample-every 1000 \
  --powers 32 --alpha 0.3 --burnin-percent 20 \
  --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
```

The model options of `rove sample` apply: `--model`, `--conditioning`, `--partitioning`, `--gaps`, `--dating`, `--monophyly`, `--weight`, `--tuning`, `--no-tuning` and `--backend`. To compare models, run the command once per model on the same matrix with the same constraints and subtract the estimates. `--sample-from-prior`, parallel chains, checkpoints and stopping diagnostics are not available.

`--powers` is the number of likelihood exponents, including zero and one. They are quantiles of a Beta(α, 1) distribution at equal spacing, `β_i = (i / (powers − 1))^(1/α)`, so a smaller `--alpha` places more exponents near zero, where the mean log-likelihood changes fastest. The values above match the conventions of BEAST 2's PathSampler. Each power runs `--steps` proposals and logs every `--sample-every`; the first `--burnin-percent` of each power's samples, by count, are discarded. The chain runs from power one down to zero, and each power continues from the previous one's final state, keeping its proposal tuning.

## Output

The output directory holds `run.json` with the model, the schedule and the operator counts, `marginal.json` with the estimates, and one `power-NN/trace.log` per power. Those traces carry the untempered posterior, likelihood and prior columns of an ordinary trace; the exponent only entered the acceptance ratio. Trees are not written.

`marginal.json` reports two estimates with standard errors:

- **Stepping stone** ([Xie et al. 2011](https://doi.org/10.1093/sysbio/syq085)) sums, over adjacent powers, the log of the mean of `L^(β_k − β_{k−1})` under the lower power. Its standard error adds the delta-method variance of each term, using the effective sample size of that term's weights.
- **Path sampling** ([Lartillot and Philippe 2006](https://doi.org/10.1080/10635150500433722)) integrates the mean log-likelihood over the exponent with the trapezoidal rule. Its standard error adds the variance of each mean, computed from its effective sample size. The error of the trapezoidal rule itself is not included, so the two estimates can differ by more than their standard errors when the schedule is coarse.

For every power the file records the retained sample count, the mean and standard deviation of the log-likelihood, its ESS and whether the autocorrelation lag limit was reached. Low ESS near power zero is common: those chains explore the prior, where trees move freely and the likelihood varies widely. More steps per power, or more powers, reduce the error.

## Reading the result

Following [Kass and Raftery (1995)](https://doi.org/10.1080/01621459.1995.10476572), a log Bayes factor above about 3 to 5 is usually read as strong support for the first model. Compare only estimates for the same matrix, the same taxa and the same constraints, and repeat the estimate with other seeds before reading a difference smaller than a few standard errors. The marginal likelihood depends on the priors, including the birth–death and clock priors, so a Bayes factor compares models together with their priors.

Mk and Mkv define likelihoods for different observation processes: Mkv conditions on every character being variable. A Bayes factor between them compares those processes as well as the models; see [character selection](./character-selection).

Under fossil tip dating with a fixed process, the FBD density omits a normalizer that depends on the process settings, the tips and the age bounds. Compare dated models only when those settings are identical, so that the constant cancels. With estimated process parameters the joint sampling target includes observation factors whose normalization has not been derived for model comparison; such comparisons are not supported. See [fossil tip dating](./tip-dating).

## Validation

Tests check both estimators on a conjugate Gaussian model with exact samples, including the calibration of their standard errors over forty replicates, and against a quadrature on a three-taxon tree with one free node height, computed from the engine's own densities. A chain at power zero reproduces the prior-sampling chain step by step while still evaluating the likelihood; exponents outside [0, 1] are rejected, and checkpoints record the live exponent. The [comparison with BEAST 2.7.8](https://github.com/getcortexlabs/rove/blob/main/benchmarks/marginal-likelihood/2026-10-31/README.md) ran MODEL_SELECTION 1.6.2's PathSampler and `rove marginal` on the strict-clock Mkv and Mk configurations of the supplied matrix, with 32 powers, 1,000,000 steps per power and three seeds per engine. Every Rove–BEAST pair of stepping-stone estimates agreed within 2.6 combined standard errors, with standard errors between 0.20 and 0.35 log units, and path sampling agreed the same way; one implementation recomputed both engines' estimates from their logs. This establishes agreement between implementations of the same estimator on one small matrix, not model-selection performance on empirical data.
