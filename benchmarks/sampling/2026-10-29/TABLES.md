# Recorded sampling results

Generated with `python3 tools/sampling/tables.py benchmarks/sampling/2026-10-29`. Ratios above one favor the joint candidate.

## Primary efficiency and regressions

Primary score: **6.626**, 90% whole-chain bootstrap interval **5.894–7.321**. Scientific gates pass: **True**. Promote to default: **False**.

Four chains per schedule give limited uncertainty resolution. The score is the geometric mean across cases of the smaller posterior/sigma median ESS/second ratio, not a wall-time speedup.

| Case | Posterior ratio [90% interval] | Sigma ratio [90% interval] |
| --- | ---: | ---: |
| reference-mk | 12.322 [11.535–14.939] | 11.115 [10.354–11.896] |
| reference-mkv | 0.938 [0.419–1.307] | 10.807 [10.331–11.502] |
| control | 13.029 [11.418–15.484] | 9.832 [8.708–13.399] |
| homoplasy | 13.187 [10.966–16.338] | 9.947 [8.683–13.605] |
| missing | 9.422 [8.002–10.769] | 10.229 [9.684–13.971] |
| reduced | 10.526 [9.596–11.838] | 8.807 [8.165–10.187] |

These median regressions exceed the predeclared 20% limit:

| Case | Parameter | Candidate / classical [90% interval] |
| --- | --- | ---: |
| reference-mkv | birth_death | 0.632 [0.287–0.905] |
| control | gamma_shape | 0.689 [0.611–1.127] |
| control | gamma_prior | 0.689 [0.611–1.127] |
| missing | gamma_shape | 0.661 [0.497–1.252] |
| missing | gamma_prior | 0.661 [0.497–1.252] |

`gamma_prior` is the negative gamma shape under its Exponential(mean=1) prior; these two rows have the same ESS and are not independent findings.

## Process wall time

Seconds, ranges across four chains. Startup, adaptation and file output are included. BEAST values are archived observations from the earlier ORC comparison.

| Case | Steps per chain | Classical Rove | Joint candidate | Archived BEAST |
| --- | ---: | ---: | ---: | ---: |
| reference-mk | 10M | 16.01–16.47 | 17.27–17.60 | 51.86–53.82 |
| reference-mkv | 40M | 69.76–75.38 | 75.00–81.39 | 211.06–215.28 |
| control | 10M | 16.13–18.39 | 17.34–22.09 | 47.87–49.14 |
| homoplasy | 10M | 20.66–21.71 | 22.03–22.47 | 58.65–59.49 |
| missing | 10M | 25.82–27.09 | 27.77–28.79 | 71.42–77.29 |
| reduced | 10M | 15.75–19.52 | 16.98–18.03 | 46.78–47.27 |

## Candidate / current classical Rove

Median ESS/second ratios for every continuous statistic. Individual chain values and classical-comparison intervals are in the JSON archive.

| Parameter | reference-mk | reference-mkv | control | homoplasy | missing | reduced |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| posterior | 12.322 | 0.938 | 13.029 | 13.187 | 9.422 | 10.526 |
| likelihood | 1.044 | 0.894 | 0.943 | 1.034 | 1.303 | 1.290 |
| prior | 12.169 | 0.954 | 11.810 | 12.149 | 9.223 | 9.821 |
| birth_death | 2.040 | 0.632 | 3.247 | 2.537 | 3.097 | 2.536 |
| birth_difference_rate | 0.892 | 0.850 | 1.240 | 1.196 | 1.109 | 1.257 |
| relative_death_rate | 0.883 | 0.939 | 1.059 | 0.988 | 0.811 | 0.928 |
| tree_height | 1.864 | 0.932 | 2.487 | 2.202 | 2.816 | 2.506 |
| tree_length | 2.070 | 0.924 | 3.283 | 2.635 | 3.463 | 2.604 |
| gamma_shape | 1.110 | 0.919 | 0.689 | 0.861 | 0.661 | 0.982 |
| clock_sigma | 11.115 | 10.807 | 9.832 | 9.947 | 10.229 | 8.807 |
| gamma_prior | 1.110 | 0.919 | 0.689 | 0.861 | 0.661 | 0.982 |
| clock_rate_prior | 12.814 | 14.445 | 12.801 | 13.662 | 8.369 | 10.447 |
| clock_sigma_prior | 14.127 | 15.323 | 15.230 | 14.203 | 11.015 | 12.021 |
| mean_branch_rate | 2.172 | 2.507 | 3.427 | 2.605 | 3.045 | 2.893 |

## Candidate / archived BEAST ORC

Median ESS/second ratios for every continuous statistic. Individual chain values and classical-comparison intervals are in the JSON archive.

| Parameter | reference-mk | reference-mkv | control | homoplasy | missing | reduced |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| posterior | 4.694 | 8.130 | 4.459 | 4.432 | 4.376 | 4.494 |
| likelihood | 3.127 | 2.392 | 2.233 | 2.145 | 2.322 | 2.512 |
| prior | 4.553 | 8.000 | 4.165 | 4.166 | 4.094 | 4.160 |
| birth_death | 2.471 | 8.299 | 2.460 | 2.584 | 2.525 | 2.362 |
| birth_difference_rate | 2.461 | 2.962 | 2.724 | 2.924 | 2.964 | 2.858 |
| relative_death_rate | 2.606 | 10.915 | 2.928 | 3.063 | 3.166 | 3.008 |
| tree_height | 2.338 | 8.053 | 2.408 | 2.391 | 2.533 | 2.274 |
| tree_length | 2.436 | 8.202 | 2.446 | 2.574 | 2.695 | 2.249 |
| gamma_shape | 2.941 | 2.994 | 1.960 | 2.563 | 2.138 | 3.041 |
| clock_sigma | 4.655 | 4.186 | 3.781 | 4.194 | 4.884 | 4.307 |
| gamma_prior | 2.941 | 2.994 | 1.960 | 2.563 | 2.138 | 3.041 |
| clock_rate_prior | 4.529 | 4.334 | 4.304 | 4.211 | 3.601 | 3.982 |
| clock_sigma_prior | 4.875 | 4.552 | 5.094 | 4.768 | 4.737 | 4.814 |
| mean_branch_rate | 1.918 | 1.951 | 1.646 | 1.876 | 1.922 | 1.891 |

## Scientific agreement

Maxima include both candidate comparisons. The ESS minimum and R-hat maximum include all continuous statistics in all three groups. The ingroup constraint is excluded from clade agreement.

| Case | Largest absolute mean z | Largest clade difference | Largest clade MCSE | Largest R-hat | Lowest ESS | All gates |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| reference-mk | 1.042 | 0.00189 | 0.00176 | 1.00398 | 333.3 | Pass |
| reference-mkv | 2.085 | 0.00001 | 0.00001 | 1.00107 | 358.9 | Pass |
| control | 1.519 | 0.00192 | 0.00198 | 1.00265 | 281.5 | Pass |
| homoplasy | 2.104 | 0.00358 | 0.00259 | 1.00255 | 262.6 | Pass |
| missing | 2.953 | 0.00517 | 0.00352 | 1.00231 | 304.9 | Pass |
| reduced | 1.722 | 0.00605 | 0.00293 | 1.00149 | 394.9 | Pass |

## Trees and autocorrelation

Distinct sampled topologies, ranges across chains. Counts alone do not establish exploration. Rooted RF, reference-topology frequencies, ESS and limited/constant indicators are retained in each comparison JSON.

| Case | Classical | Candidate | Archived BEAST | Posterior autocorrelation time: classical → candidate | Sigma autocorrelation time: classical → candidate |
| --- | ---: | ---: | ---: | ---: | ---: |
| reference-mk | 137–158 | 139–170 | 138–158 | 19.01 → 1.41 | 19.08 → 1.59 |
| reference-mkv | 2–6 | 2–6 | 2–4 | 17.68 → 18.51 | 18.67 → 1.58 |
| control | 157–169 | 149–167 | 148–167 | 22.53 → 1.56 | 23.07 → 2.00 |
| homoplasy | 301–311 | 279–332 | 285–308 | 22.85 → 1.64 | 22.05 → 2.06 |
| missing | 1119–1186 | 1131–1144 | 1138–1164 | 16.37 → 1.64 | 21.46 → 1.92 |
| reduced | 256–275 | 250–276 | 252–257 | 18.60 → 1.67 | 21.50 → 2.35 |

Autocorrelation times are retained sample count divided by ESS, in logged samples (one per 1,000 steps). The table uses medians across chains. Lag-one autocorrelation is also recorded in the JSON.
