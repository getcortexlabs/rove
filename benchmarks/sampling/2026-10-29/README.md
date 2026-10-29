# Relaxed-clock proposal comparison

This experiment follows the fixed [proposal evaluation protocol](../../../fixtures/validation/SAMPLING-EFFICIENCY.md)
for [#27](https://github.com/getcortexlabs/rove/issues/27). The probability
model, likelihood kernel, constraints and inference criteria are unchanged.

## Exploratory selection

The pilot used reference-mk, reference-mkv and missing, two seeds per schedule,
two million steps per chain and 10% burn-in. Five schedules ran sequentially
on an Apple M4 Max. The score is the geometric mean across cases of the
smaller of the posterior and clock-sigma median ESS/second ratios against
classical Rove. These short runs select a candidate; they do not establish
posterior concordance or a final speedup.

| Candidate | Pilot score |
| --- | ---: |
| sigma-frequency | 1.005 |
| time-rates | 1.510 |
| sigma-rates | 6.437 |
| combined | 7.397 |

`combined` is selected without further weight adjustment. Its overrides are
`node_height=18`, `node_time_rates=12`, `root_scale=1`, `root_time_rates=2`,
`clock_sigma_scale=1`, `clock_sigma_rates=2`. All other weights keep their
classical values; the total selection weight is unchanged.

[pilot.json](pilot.json) retains every parameter of every pilot chain,
including regressions. [pilot-metadata.json](pilot-metadata.json) records
commands, timings, hashes, model settings and operator counts. The pilot
used an uncommitted implementation: its source-file hashes are recorded,
and final evaluation requires a clean commit. No default is changed on the
basis of these pilot observations.

The joint time/rate and standardized-rate proposals implement established
ideas from [Douglas, Zhang and Bouckaert (2021)](https://doi.org/10.1371/journal.pcbi.1008322).
Their independently derived transformations and Hastings factors are documented
in the [reference model guide](../../../docs/guide/reference-model.md#joint-proposals).
This is not an implementation of ORC's entire adaptive mixture.
