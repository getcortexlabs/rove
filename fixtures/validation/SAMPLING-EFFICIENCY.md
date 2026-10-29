# Relaxed-clock proposal evaluation

This protocol separates exploratory selection from the final comparison for
#27. The probability model, likelihood kernel, constraints and existing
inference criteria in [PROTOCOL.md](PROTOCOL.md) remain fixed.

## Candidates and pilot

Profile the classical schedule after two million warm-up steps: 500,000 timed
steps, then 100,000 steps with allocation tracking. Attribute complete step
cost, acceptance, invalid proposals, squared parameter jumps and changes of
canonical clades to each move. Topology comparisons and allocation tracking
stay outside the timing region. This is a descriptive profile, not inference
throughput. Relate it to retained-trace correlations and ESS from the archive.

Evaluate four alternatives to the classical schedule. All keep its total
selection weight and leave topology-changing moves and gamma-shape updates
unchanged:

| Candidate | Changes to classical selection weights |
| --- | --- |
| sigma-frequency | node_height 21; clock_sigma_scale 12 |
| time-rates | node_height 18; node_time_rates 12; root_scale 1; root_time_rates 2 |
| sigma-rates | clock_sigma_scale 1; clock_sigma_rates 2 |
| combined | Union of time-rates and sigma-rates changes |

Time/rate moves preserve incident branch distances; the sigma/rate move
preserves standardized log rates. Derive their Hastings factors, test reverse
maps, validate independently computable targets and rejected-state restoration
before using them in an inference pilot. No adaptive mixture or Bactrian kernel
is introduced in this experiment.

Pilot the five schedules on reference-mk, reference-mkv and missing, using
seeds 27011 and 27012, two million steps, sampling every 200, and 10% burn-in.
Pilot ESS can be poorly resolved; it does not establish posterior agreement.
Select one candidate by the geometric mean across these cases of the smaller
of its posterior-density and clock-sigma median ESS/second ratios against
classical. Undefined primary ESS disqualifies a candidate. Keep every pilot,
including regressions. The pilot seeds are never used for the final inference
or efficiency claims.

## Final comparison

Freeze the selected weights and implementation in a clean commit. Run current
classical and selected Rove schedules for the six archived cases with four
seeds per schedule. Use 20261110–20261113 and 10M steps for the five Mk cases;
use 20261210–20261213 and 40M for reference-mkv. Sample every 1,000 and discard
10%. Alternate schedule order by seed; execute timed chains sequentially.
Record the command, build flags, source/binary/input hashes, hardware, load,
power observations and operator tuning/counts. Diagnostics run after timing.

Use the four archived BEAST 2.7.8 ORC runs per case, with their original hashes,
settings, runtime observations and diagnostics. They are the recorded external
reference, not fresh simultaneous timing measurements. Report that distinction.
Check that current classical Rove reproduces the archived scientific samples
where executable arithmetic permits; never expect candidate trajectories to
match classical or BEAST trajectories merely because seed labels match.

Apply the unchanged mean, R-hat, ESS, clade-frequency and clade-uncertainty gates
from PROTOCOL.md to candidate versus classical and candidate versus BEAST.
Report topology movement and limited/constant indicators separately. If a new
convergence or precision failure requires extension, use its recorded 40M/160M
extension rule for all compared schedules, retaining earlier batches. A
well-resolved disagreement is a proposal/numerical failure to investigate,
not a reason to change thresholds.

Report wall time and ESS/second for every traced parameter, every chain and
every case. For uncertainty in efficiency ratios, independently resample four
chains within each case and schedule, 10,000 bootstrap replicates with seed
27099. This captures observed between-chain variability, not all possible
Monte Carlo error; only four chains give limited precision.

A candidate becomes the default only if all scientific gates pass, the final
primary score defined above exceeds 1.2 with a 90% bootstrap lower bound above
one, and no continuous parameter/case median ESS/second falls below 0.8 of
classical. Report all tradeoffs against the recorded BEAST results too. If
these conditions fail, retain the classical default and archive the rejected
candidate. An opt-in proposal must still pass the numerical and target tests.
Neither a rejected candidate nor a result on six small constructed matrices
supports a general performance claim.
