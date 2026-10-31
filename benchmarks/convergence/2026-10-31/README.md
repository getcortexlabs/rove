# Independent-chain budgets on 90 and 500 taxa

The 90 × 250 workload meets every predeclared scalar, rooted-RF and clade
criterion at 16 million steps per chain and again at 18 million. The first
sustained pass under the two-check rule is therefore **18 million per chain**.
It continues to pass at the 20-million cap. This records a larger workload
satisfying the declared diagnostics; it does not prove convergence or validate
a biological interpretation of the data. The 500 × 759 case does **not** meet
the criteria within its 5-million-step-per-chain cap: both scalar and clade
diagnostics fail at every declared prefix.

The [protocol](../PROTOCOL.md), runner and diagnostic tool were frozen in
`ee592836b5234d7af562e265ff5170438063cb3d`. The study changes no sampler defaults.
It runs the entire fixed cap, then diagnoses predetermined prefixes; it does
not benchmark the application's scalar-only automatic stopping feature.

## Configuration and timing

Four independent chains run concurrently, using seeds 2026103101–2026103104
for the public 90 × 250 Mk matrix and 2026103111–2026103114 for the private
500 × 759 Mkv matrix. The 500-taxon case starts only after all 90-taxon prefix
diagnostics have finished. Both use the reference preset, four gamma
categories, a lognormal relaxed clock, the birth–death and parameter priors,
observed-state partitions, gaps as missing, default classical moves and
tuning, no dating and no monophyly constraints.

The machine is an Apple M4 Max with 12 performance and 4 efficiency cores and
64 GB, running macOS 26.6.2. The ordinary Rust 1.99.1 / LLVM 23.1.1 release
binary uses NEON, O3, fat LTO, one codegen unit, default target features and
no experimental features. The [separate profiling report](../../profile/2026-10-31/README.md)
uses paired instrumented runs; none of those timings enters this study.

Wall time is process launch through exit of `rove sample`, including startup,
checkpoints, trace/tree output, coordination and the CLI's final comparison.
It excludes the subsequent prefix diagnostics. Checkpoints occur every
100,000 steps. The reported first passing **budget** is not an observed
early-stop runtime: every chain continues to its cap. No BEAST analysis was
timed, so there is no cross-engine speedup claim.

## 90 × 250, Mk

All four 20-million-step chains completed in **470.94 s** of group wall time
(7 min 51 s). Each chain logged 20,001 draws at intervals of 1,000 steps.
At the cap, discarding floor(10%) leaves 18,001 draws per chain.

| Prefix per chain | Minimum scalar ESS | Maximum scalar R-hat | Largest clade-frequency spread | Largest per-chain clade MCSE | All criteria |
| --- | ---: | ---: | ---: | ---: | --- |
| 2M | 12.3 | 1.03163 | 0.0522 | 0.03707 | not met |
| 4M | 49.4 | 1.02481 | 0.0403 | 0.02320 | not met |
| 6M | 77.7 | 1.01276 | 0.0411 | 0.01861 | not met |
| 8M | 102.9 | 1.01120 | 0.0289 | 0.01590 | not met |
| 10M | 113.4 | 1.01348 | 0.0252 | 0.01554 | not met |
| 12M | 156.4 | 1.00480 | 0.0233 | 0.01467 | not met |
| 14M | 174.3 | 1.00276 | 0.0286 | 0.01385 | not met |
| 16M | 194.5 | 1.00256 | 0.0283 | 0.01178 | pass |
| 18M | 223.9 | 1.00295 | 0.0261 | 0.01002 | pass |
| 20M | 249.5 | 1.00161 | 0.0273 | 0.00886 | pass |

At the 20M cap, ESS/time sums the four within-chain ESS estimates and divides
by group wall time. It describes this complete four-chain run and excludes
the later prefix diagnostics; it is not pooled bulk/tail ESS or the elapsed
time of an early stop.

| Statistic | ESS in chains 1 / 2 / 3 / 4 | R-hat | Summed ESS / group s |
| --- | ---: | ---: | ---: |
| posterior | 347.0 / 374.6 / 315.2 / 367.4 | 1.00096 | 2.982 |
| likelihood | 1245.3 / 1129.3 / 1065.8 / 1401.0 | 1.00023 | 10.280 |
| prior | 277.8 / 297.5 / 262.5 / 305.8 | 1.00159 | 2.428 |
| birth_death | 739.1 / 634.2 / 808.9 / 736.3 | 1.00024 | 6.197 |
| birth_difference_rate | 3897.3 / 4248.8 / 5021.1 / 4931.6 | 0.99998 | 38.432 |
| relative_death_rate | 11531.8 / 12186.1 / 11470.4 / 11412.9 | 0.99998 | 98.954 |
| tree_height | 1568.5 / 1347.7 / 1538.8 / 1359.1 | 1.00017 | 12.346 |
| tree_length | 667.2 / 580.7 / 738.1 / 664.8 | 1.00029 | 5.629 |
| gamma_shape | 2426.8 / 2410.9 / 2032.3 / 2491.5 | 1.00067 | 19.878 |
| gamma_prior | 2426.8 / 2410.9 / 2032.3 / 2491.5 | 1.00067 | 19.878 |
| clock_sigma | 311.6 / 329.5 / 304.1 / 334.1 | 1.00106 | 2.716 |
| clock_rate_prior | 282.7 / 307.4 / 249.5 / 311.7 | 1.00161 | 2.445 |
| clock_sigma_prior | 295.5 / 313.5 / 286.2 / 332.4 | 1.00095 | 2.606 |
| mean_branch_rate | 1046.8 / 782.5 / 1025.6 / 905.9 | 1.00072 | 7.986 |

Scalar criteria first pass at 12M, but four clades still exceed the MCSE
threshold of 0.0125. Two still exceed it at 14M. At 16M, 18M and 20M, every
scalar, RF and clade criterion passes. A scalar-only stopping rule would not
have tested this remaining uncertainty in clade frequencies.

At 20M there are 344 distinct observed clades; 126 have pooled frequency at
least 0.01. Their largest frequency spread across chains is 0.02733 and the
largest per-chain batch MCSE is 0.00886. Twelve clades are constant within
every chain; agreement on them alone does not establish exploration.
The 20 equal batches use 900 draws each and omit one tail draw per chain.

The retained chains contain 5,827 / 5,969 / 6,115 / 6,039 distinct topologies.
Rooted-RF ESS values are 9,205.7 / 6,152.5 / 9,656.5 / 9,541.3, with
R-hat 1.00013. Their mean RF distances to the initial reference tree are all
near 174, close to the maximum of 176 for these 90-tip binary trees. Distance
to that remote reference is insensitive to many local rearrangements: RF
passes even at 2M while scalar and clade criteria fail. This is why all three
diagnostic groups are required.

The slowest scalar at the cap is `clock_rate_prior`, with ESS 249.5 in chain 3;
the gamma-shape ESS is 2,032–2,491. This workload does not support assuming that
gamma is always the limiting parameter. Wide-exchange acceptance is only
0.033–0.037% and Wilson–Balding acceptance 0.101–0.110%, yet this case meets the
declared gates. Low acceptance alone does not identify the cause of poor mixing.

## 500 × 759, Mkv

The four 5-million-step chains completed in **2682.55 s** of group wall
time (44 min 43 s). Each chain logged 10,001 draws; 9,001 remain after the
declared burn-in. **No prefix meets the criteria.** This workload did not
meet the criteria within the budget; the experiment does not establish that
it cannot converge with more computation or different proposals.

| Prefix per chain | Minimum scalar ESS | Maximum scalar R-hat | Largest clade-frequency spread | Largest per-chain clade MCSE | All criteria |
| --- | ---: | ---: | ---: | ---: | --- |
| 1M | 2.4 | 2.04158 | 1.0000 | 0.11446 | not met |
| 2M | 2.8 | 2.10745 | 1.0000 | 0.11402 | not met |
| 3M | 3.7 | 1.73921 | 1.0000 | 0.11413 | not met |
| 4M | 4.8 | 1.72039 | 1.0000 | 0.11443 | not met |
| 5M | 5.5 | 1.63939 | 1.0000 | 0.11371 | not met |

At the 5M cap, a dagger marks an ESS estimate that reaches the estimator's
lag limit and therefore fails the protocol. ESS/time is only descriptive for
these chains: a finite estimate does not rescue their cross-chain disagreement.

| Statistic | ESS in chains 1 / 2 / 3 / 4 | R-hat | Summed ESS / group s |
| --- | ---: | ---: | ---: |
| posterior | 5.5† / 8.0† / 9.4 / 6.6† | 1.63939 | 0.011 |
| likelihood | 5.5† / 8.0† / 9.5 / 6.5† | 1.60843 | 0.011 |
| prior | 6.8† / 9.1 / 9.6† / 19.6 | 1.11443 | 0.017 |
| birth_death | 17.9 / 9.8 / 12.8 / 27.3 | 1.07284 | 0.025 |
| birth_difference_rate | 31.4 / 23.9 / 29.2 / 63.0 | 1.03547 | 0.055 |
| relative_death_rate | 3578.6 / 3541.4 / 3302.5 / 3433.0 | 1.00062 | 5.165 |
| tree_height | 12.1 / 14.7 / 8.6 / 21.1 | 1.15977 | 0.021 |
| tree_length | 18.5 / 10.0 / 12.9 / 26.9 | 1.07270 | 0.025 |
| gamma_shape | 555.3 / 483.9 / 632.1 / 507.9 | 1.00937 | 0.812 |
| gamma_prior | 555.3 / 483.9 / 632.1 / 507.9 | 1.00937 | 0.812 |
| clock_sigma | 12.0 / 9.4 / 14.2 / 17.7 | 1.10583 | 0.020 |
| clock_rate_prior | 6.3† / 6.7 / 7.5† / 15.1 | 1.13525 | 0.013 |
| clock_sigma_prior | 13.0 / 9.5 / 14.9 / 17.4 | 1.10583 | 0.020 |
| mean_branch_rate | 7.8 / 7.4† / 7.3† / 11.3 | 1.13533 | 0.013 |

Only relative death rate and the two gamma columns meet their scalar gates
at the cap. Posterior R-hat is 1.63939 and likelihood R-hat is 1.60843; their
ESS estimates are roughly 5–10, with three chains reaching the lag limit.
Clock sigma has ESS 9.4–17.7 and R-hat 1.10583. The evidence does not point to
gamma shape alone as the limiting parameter.

Of 9,331 distinct clades observed after burn-in, 2,132 meet the pooled 0.01
frequency threshold. Of those, 1,450 exceed the cross-chain spread limit and
1,905 exceed the batch-MCSE limit. The maximum frequency spread is **1.0**:
at least one group is present in every retained tree of one chain and absent
from every retained tree of another. Maximum MCSE per chain is
0.11254 / 0.11371 / 0.11265 / 0.11248, against a limit of 0.0125. Twenty batches
of 450 draws omit one tail draw per chain. Forty-five clades are constant
within each chain; that does not imply agreement between chains.

There are 9,000 / 9,000 / 9,001 / 9,001 distinct retained topologies. Nevertheless,
all four RF series are constant at 996, the maximum distance to the initial
500-tip reference tree, so their ESS and R-hat are unavailable and fail the
gates. This does **not** mean the sampled trees are identical: it means that
distance to this remote reference cannot distinguish their changes. The
clade-frequency disagreement independently rules out a passing assessment.

Operator outcomes over the complete budget, including initial exploration:

| Move | Attempts per chain, min–max | Accepted / attempted, min–max | Failed / attempted, min–max |
| --- | ---: | ---: | ---: |
| node_height | 1,482,765–1,486,523 | 34.007–34.095% | 0.000–0.000% |
| subtree_slide | 740,410–741,931 | 23.313–23.355% | 14.726–14.920% |
| narrow_exchange | 741,203–742,528 | 13.903–14.249% | 0.000–0.000% |
| wide_exchange | 148,044–148,860 | 0.092–0.128% | 50.004–50.225% |
| wilson_balding | 148,257–148,607 | 0.283–0.306% | 1.082–1.168% |
| root_scale | 147,599–148,500 | 23.268–23.657% | 7.203–17.748% |
| tree_scale | 148,109–148,372 | 23.326–23.394% | 0.000–0.000% |
| birth_difference_scale | 148,082–148,671 | 23.348–23.606% | 0.000–0.000% |
| relative_death_scale | 147,997–148,592 | 23.417–23.537% | 0.034–0.185% |
| gamma_shape_scale | 4,874–5,062 | 22.876–23.302% | 0.000–0.000% |
| clock_sigma_scale | 147,897–148,803 | 23.288–23.377% | 0.000–0.000% |
| branch_rate_scale | 988,926–990,328 | 23.512–23.519% | 0.000–0.000% |

Wide exchange accepts 137–189 proposals per chain and Wilson–Balding 421–454.
Branch-rate and clock-sigma proposals accept about 23%, yet clock-related
statistics still mix poorly. Both tree-space agreement and rate/tree scalars
remain unresolved. These observations motivate controlled proposal-schedule
or coupled-chain experiments; acceptance rates and phase occupancy alone
cannot identify which move causes the failure. No move weights were changed
or selected from this study.

The machine remained on AC power. The 500-taxon group's one-minute load was
16.03 before and 15.85 after; thermal-status queries were unavailable. Timing
is a record of this shared machine, not a pinned-core or dedicated-host result.

The [terminal-state profile](../../profile/2026-10-31/README.md#cost-from-the-terminal-inference-states)
measures short, separate copies of chains 1 and 2 after the 20M / 5M caps.
Node partials plus rescaling still dominate the output-free sampling windows
(70.97% / 83.70%). Those continuations do not enter any diagnostic above.

## Diagnostics and reproduction

Every prefix discards floor(10%) of its draws, including state zero in the
count. Every sampled scalar and the rooted-RF series must have ESS ≥ 100 in
each of four chains and rank-normalized split R-hat ≤ 1.01. Unavailable,
constant or lag-limited diagnostics fail. This is summed within-chain ESS,
not pooled bulk or tail ESS. Every clade with pooled frequency ≥ 0.01 must
have a cross-chain frequency range ≤ 0.05 and per-chain 20-batch MCSE ≤ 0.0125.
Two consecutive passing prefixes are required; an intervening failure resets
the count. These checks remain diagnostics rather than proof of stationarity.

[results.json](results.json) retains every per-parameter, per-chain diagnostic
at every prefix, operator statistics, commands, hashes and power/load records.
Follow the [frozen protocol](../PROTOCOL.md#commands-and-publication) to repeat
the study. The public matrix is in the repository; the 500-taxon case requires
the exact private input fingerprint. Private labels, alignments, sampled
trees, clade membership maps and checkpoints remain local. ESS/time from a
failing chain is descriptive and cannot establish a time-to-answer winner.
