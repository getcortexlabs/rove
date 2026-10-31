# Gamma proposal frequency

Issue #42 compares the existing `gamma_shape_scale` move at selection weights
0.1 and 3. No density, support, discretization, Hastings ratio or other move
changes. Proposal-size adaptation remains enabled. This protocol is frozen
before the final runs; its SHA-256 is recorded with them.

## Pilot and hypothesis

The four-tip dated example gave only about 8,100–9,200 gamma attempts in 10M
steps. Acceptance was near the adaptation target, 0.234. The candidate gives
this parameter more opportunities to move. A fixed mixture of invariant MH
kernels remains invariant when its state-independent selection weights change.

Exploration used gamma weights 0.1, 0.5, 1 and 3, two dated configurations,
1M steps and samples every 100. Independent four-chain groups start at seeds
42011 and 42021. An earlier group starting at 42012 overlaps three seeds with
42011; retain it as exploratory data, not independent replication. Reference
pilots use Mk, Mkv and missing-data cases, weights 0.1 and 3, 2M steps, samples
every 200, and groups starting at 42031 and 42041. These select weight 3 but
cannot support a final efficiency or posterior-agreement claim.

## Final runs

- Use one frozen release executable for both schedules, with explicit weight
  overrides. Record source, executable, inputs, commands, compiler, hardware,
  output hashes and actual operator counts. Do not change the executable
  during timing.
- Keep the existing reference preset, priors, starting values and conditioning.
  The six undated validation cases retain their T1–T19 monophyly constraint.
  The two dated cases use `dating/dating.json` and `fbd-estimation/dating.json`.
- Four concurrent independent chains per group; run groups sequentially,
  alternating schedule order by case/repetition. No separate warmup. External
  group wall time includes startup, adaptation, IO, checkpoints and the final
  Rove comparison. Do not interpret this as a single-chain latency measurement.
- Dated cases: two independent groups per schedule, 10M steps each, starting
  seeds 42101 and 42111 for fixed FBD, 42121 and 42131 for estimated FBD.
- Undated cases: one group per schedule, four independent chains, 10M steps
  except reference-Mkv at 40M. Group seeds 42201, 42211, 42221, 42231, 42241,
  42251 for reference-Mk, reference-Mkv, control, homoplasy, missing, reduced.
- Samples every 1,000 steps, discard 10% by sample count, including state zero
  in the total. No outcome-dependent extensions, dropped seeds or retuning.

## Decision

Check fixed-state numerical tests and the existing known-prior and cache
restoration tests before changing a default. Independently summarize traces
and trees with the existing BEAST 2 validation helper after timing; BEAST is
not involved in Rove sampling. Compare all sampled scalar means within four
combined Monte Carlo standard errors, using the larger of within-chain and
between-chain mean variances. Require rank-normalized split/folded R-hat at
most 1.01, every sampled scalar ESS at least 100 and total ESS at least 400.
Unavailable diagnostics or an ESS lag limit prevent promotion.

For every unconstrained clade with pooled frequency at least 0.01 in either
schedule, require mean-frequency difference at most 0.05 and MCSE at most
0.0125 in each schedule. Clade MCSE uses 20 contiguous batches per chain and
the between-chain safeguard. Also report rooted-RF and topology-indicator
diagnostics; constant tree indicators cannot establish topology mixing.
The dated four-tip anchor tree is only a diagnostic reference, not a known
generating topology or a recovery claim.

For efficiency, divide each chain's ESS by its group's wall time. Require a
median gamma ESS/time gain greater than 2 in both dated cases and no sampled
scalar's median ESS/time ratio below 0.8. Report the minimum ESS across all
sampled scalars, runtime, tails, acceptance and correlations as well. Timings
within a group are shared; four chains are not four independent timing trials.

Promote globally only if all eight cases pass the scientific and non-regression
checks. If only the dated cases pass, scope the change to dated reference
analyses and preserve undated schedules exactly. If a dated case fails, keep
the candidate opt-in and report the failure. Do not change priors or thresholds
after observing the final results. These small constructed cases do not
establish performance on empirical fossil data, M1 hardware or large matrices.
