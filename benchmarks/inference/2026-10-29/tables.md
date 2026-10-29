## Agreement and convergence

| Case | Steps per chain | Largest absolute mean z | Largest clade difference | Largest continuous R-hat | Lowest continuous ESS | Criteria |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| reference-mkv | 40,000,000 | 1.313 | 0.00001 | 1.00107 | 358.9 | Pass |
| reference-mk | 10,000,000 | 1.489 | 0.00108 | 1.00398 | 333.3 | Pass |
| control | 10,000,000 | 1.270 | 0.00244 | 1.00265 | 281.5 | Pass |
| homoplasy | 10,000,000 | 2.299 | 0.00161 | 1.00145 | 262.6 | Pass |
| missing | 10,000,000 | 3.200 | 0.00536 | 1.00231 | 304.9 | Pass |
| reduced | 10,000,000 | 1.651 | 0.00350 | 1.00140 | 394.9 | Pass |

These are the last batches required by the protocol. Earlier batches remain
archived below; batches of different lengths are not pooled. The clade maximum
excludes the imposed T1–T19 group. Each case has four chains per engine.

| Batch | Means agree | Clades agree | Tree means agree | Convergence gates | Clade precision |
| --- | --- | --- | --- | --- | --- |
| reference-mkv, 10M | yes | yes | yes | no | yes |
| reference-mkv, 40M | yes | yes | yes | yes | yes |
| reference-mk, 10M | yes | yes | yes | yes | yes |
| control, 10M | yes | yes | yes | yes | yes |
| homoplasy, 10M | yes | yes | yes | yes | yes |
| missing, 10M | yes | yes | yes | yes | yes |
| reduced, 10M | yes | yes | yes | yes | yes |

## Trees

| Case | Distinct topologies per chain, Rove | BEAST | Mean rooted RF, Rove | BEAST | Reference topology frequency, Rove | BEAST |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| reference-mkv | 2–6 | 2–4 | 0.0002 | 0.0001 | 0.99991 | 0.99993 |
| reference-mk | 137–158 | 138–158 | 0.4884 | 0.4901 | 0.77719 | 0.77675 |
| control | 157–169 | 148–167 | 0.4813 | 0.4657 | 0.78289 | 0.78925 |
| homoplasy | 301–311 | 285–308 | 0.7811 | 0.7736 | 0.67398 | 0.67412 |
| missing | 1119–1186 | 1138–1164 | 2.4972 | 2.4988 | 0.26622 | 0.26355 |
| reduced | 256–275 | 252–257 | 1.2414 | 1.2339 | 0.49714 | 0.49786 |

A constant topology indicator has no measurable mixing ESS. The comparison
files flag tree diagnostics with any constant chain as limited; they are
not counted as convergence evidence. All chains obeyed the imposed ingroup
constraint. Unconstrained reference clades and their Monte Carlo errors are
listed separately in each comparison file.

## Runtime and sampling efficiency

| Case | Wall seconds per chain, Rove | BEAST | BEAST/Rove median wall time |
| --- | ---: | ---: | ---: |
| reference-mkv | 67.78–68.34 | 211.06–215.28 | 3.13 |
| reference-mk | 15.76–16.28 | 51.86–53.82 | 3.30 |
| control | 15.86–16.15 | 47.87–49.14 | 3.05 |
| homoplasy | 20.11–20.52 | 58.65–59.49 | 2.92 |
| missing | 25.68–26.22 | 71.42–77.29 | 2.84 |
| reduced | 15.74–16.05 | 46.78–47.27 | 2.98 |

The following ratios divide median ESS/second in Rove by median ESS/second
in BEAST. Values above one favor Rove for that statistic. They include the
cost of the different proposal schedules and tuning rules, not just pruning.

| Case | Log-posterior density | Log likelihood | Gamma shape | Clock sigma | Tree height |
| --- | ---: | ---: | ---: | ---: | ---: |
| reference-mkv | 9.27 | 2.77 | 3.39 | 0.40 | 9.32 |
| reference-mk | 0.38 | 3.08 | 2.73 | 0.42 | 1.29 |
| control | 0.37 | 2.55 | 3.11 | 0.42 | 1.05 |
| homoplasy | 0.35 | 2.15 | 3.12 | 0.44 | 1.13 |
| missing | 0.49 | 1.87 | 3.32 | 0.47 | 0.94 |
| reduced | 0.44 | 1.96 | 3.09 | 0.51 | 0.91 |

Per-chain ESS, wall times, ESS/second and their variation remain in the JSON
files. A faster chain can have lower ESS/second for a parameter that it
explores less efficiently.
