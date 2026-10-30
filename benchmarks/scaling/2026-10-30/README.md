# Power-of-two scaling and subtree exponent totals

The new scaling reduces the median time of the 500-taxon, 759-character sampler workload from 9.54 s to 6.36 s on NEON (33.3%), and from 10.65 s to 7.68 s on the portable backend (27.9%). These are 20,000-step timing runs with minimum scalar ESS around 2.3–2.5, not completed analyses. Small reference runs take 13–17% less wall time, but their ESS per second does not consistently improve across these short chains.

## Implementations

- **before:** commit `883cb2353c1a05f9f2fb9985c1e4237bffc90780`, maximum-based rescaling and a root sum over all node log scalers.
- **power-sum:** power-of-two scaling with local integer exponents, still summed over all nodes at root integration.
- **subtree:** the production implementation: each node stores its local exponent plus the totals of its children; integration reads the root total.

For a finite positive pattern maximum `m`, the exponent is `clamp(floor(log2(m)), -1023, 1022)`, obtained from the IEEE 754 bits. Every partial is multiplied by `2^-e`. This uses the lower power rather than the nearest power proposed in #18. The bounds keep the multiplier finite and normal even for subnormal maxima. Zero and nonfinite maxima use exponent zero. The root adds `E * ln(2)` once per pattern. Integer subtree totals share the partials buffer indices, so rejection and checkpoint restoration need no separate accumulation.

## Measurement

Apple M4 Max, 12 performance and 4 efficiency cores, 64 GB RAM. The executables use the default ARM target, release/bench optimization, fat LTO and one codegen unit; no `target-cpu=native` override. This differs from the older #8 report’s native-CPU build. Hardware, Rust version, load, power observations, executable hashes and source hashes are retained in the JSON files. Background load and CPU affinity were not controlled. The final release executable is byte-identical to the measured subtree executable; signing the bundled helper changes its hash.

The existing #8 `likelihood` harness validates all 12 cases before timing all four combinations of matrix/Mk transition paths and portable/NEON backends. Each metric uses five calibrated batches of at least 5 ms and 10 ms warm-up, in three fresh processes per implementation. Variant order rotates between processes and the harness reverses its backend order on alternating repetitions. The table reports the median of process medians. All raw TSV batches are included.

### Kernel times

All rows below use the direct Mk path. Times are microseconds; the speed ratio is before/subtree.

| Case | Backend | Operation | Before | Power-sum | Subtree | Ratio |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| reference_20x50 | portable | full_cached | 4.785 | 4.155 | 4.035 | 1.186× |
| reference_20x50 | portable | tip_update | 1.695 | 1.457 | 1.327 | 1.278× |
| reference_20x50 | neon | full_cached | 3.210 | 2.439 | 2.289 | 1.402× |
| reference_20x50 | neon | tip_update | 1.225 | 0.938 | 0.795 | 1.541× |
| reference_20x50_mkv | portable | full_cached | 5.903 | 5.099 | 5.015 | 1.177× |
| reference_20x50_mkv | portable | tip_update | 1.989 | 1.785 | 1.611 | 1.235× |
| reference_20x50_mkv | neon | full_cached | 4.088 | 3.199 | 3.029 | 1.350× |
| reference_20x50_mkv | neon | tip_update | 1.459 | 1.192 | 1.051 | 1.388× |
| morphology_90x250 | portable | full_cached | 203.236 | 160.447 | 158.918 | 1.279× |
| morphology_90x250 | portable | tip_update | 22.567 | 19.895 | 13.315 | 1.695× |
| morphology_90x250 | neon | full_cached | 158.764 | 111.128 | 108.676 | 1.461× |
| morphology_90x250 | neon | tip_update | 18.889 | 16.146 | 9.375 | 2.015× |
| eight_states | portable | full_cached | 573.807 | 544.128 | 544.612 | 1.054× |
| eight_states | portable | tip_update | 52.586 | 50.902 | 43.561 | 1.207× |
| eight_states | neon | full_cached | 451.086 | 415.154 | 414.294 | 1.089× |
| eight_states | neon | tip_update | 43.777 | 41.324 | 34.233 | 1.279× |

Every measured likelihood/update/proposal median improved against the old scaling. `slower-cases.json` nevertheless retains **all** slower rows, including changes below 1%. Core construction has one extra allocation of `8 * stride` bytes for root exponents, and some setup medians rose by up to 11.5%. Unchanged parse/compress/store paths also show timing variation; this is not attributed to the scaling algorithm. Every hot evaluation, update, accept/reject and store/restore path still has zero allocation/reallocation calls.

Compared with power-sum alone, subtree totals primarily help incremental updates. Eleven full-evaluation medians were slightly slower (at most 1.2%); those rows are retained too. Summing two child exponents during node updates shifts work to pruning, while eliminating the root traversal over every node.

### Complete sampler runs

The 54 timings run sequentially in a seeded shuffled order: three seeds, two backends, three implementations and three datasets. All use the reference gamma + relaxed-clock model, observed-state partitioning and ordinary single chains. Mk and Mkv reference cases run 2,000,000 steps and log every 1,000, with the T1–T19 ingroup constrained. The supplied 500 × 759 matrix runs 20,000 steps and logs every 100 without a constraint; it has 180 binary, 140 ternary and 439 four-state characters. Its input, traces and trees remain local; only its fingerprint and aggregate measurements are published. Checkpoint policy is the unchanged default.

| Dataset | Backend | Before (s) | Power-sum (s) | Subtree (s) | Time reduction |
| --- | --- | ---: | ---: | ---: | ---: |
| mk | portable | 4.0545 | 3.7002 | 3.5192 | 13.2% |
| mk | neon | 3.2866 | 2.9554 | 2.7427 | 16.5% |
| mkv | portable | 4.3250 | 3.9989 | 3.7643 | 13.0% |
| mkv | neon | 3.5541 | 3.2343 | 2.9476 | 17.1% |
| large | portable | 10.6533 | 9.5116 | 7.6814 | 27.9% |
| large | neon | 9.5426 | 8.5650 | 6.3644 | 33.3% |

All 27 backend pairs produce identical scientific trace and tree files. All 18 power-sum/subtree pairs are also identical: caching the integer sum changes runtime, not the arithmetic result. The old maximum-based scaling has different rounding and does not produce bit-identical traces. Several small runs eventually take different MCMC paths. One subtree Mkv/NEON run took 3.716 s versus 3.583 s before for the same seed; it is retained in the median and raw records.

### Sampling quality

Diagnostics use 10% burn-in and are calculated after all timings. All traced statistics, means, ESS values, lag-limit flags, minimum ESS/time and distinct retained clade counts are in `sampler.json`. Distinct clade counts are descriptive, not convergence tests. The imposed ingroup is not evidence of independent recovery.

NEON ranges across the three seeds:

| Dataset | Version | Minimum scalar ESS | Minimum ESS/s | Distinct clades |
| --- | --- | ---: | ---: | ---: |
| mk | before | 56.36–102.26 | 17.15–30.94 | 56.00–58.00 |
| mk | subtree | 52.10–88.39 | 18.99–31.97 | 59.00–60.00 |
| mkv | before | 44.80–63.96 | 12.61–18.11 | 19.00–19.00 |
| mkv | subtree | 39.89–54.92 | 12.30–18.63 | 19.00–20.00 |
| large | before | 2.33–2.50 | 0.24–0.26 | 4919.00–6012.00 |
| large | subtree | 2.33–2.50 | 0.37–0.39 | 4919.00–6012.00 |

These runs do not establish convergence or a general gain in ESS per second. In particular, the small datasets have different ESS after rounding changes the sampled path, and the large dataset remains in its initial exploration. The optimization targets evaluation cost; it does not change proposal operators. None of these timings compares Rove with BEAST or estimates an M1 runtime.

## Numerical checks

- The existing random-tree tests cover states/partials, missing data, unequal frequencies, one to four categories, distances from 1e-9 to 100, accepted/rejected changes and backend bit identity. Special-value tests cover signed zero, subnormals, infinities and NaNs.
- A new exponent-boundary test covers every finite binary exponent and five mantissa boundary values, with finite multipliers and bitwise backend agreement.
- Thirty-six independent unscaled Decimal evaluations retain 60 significant digits, including 2/3/4/8 states and balanced/ladder trees with 2,048 tips. Maximum Mk-total error changed from `1.61e-10` before to `9.10e-13` after. With Mkv on 1e-9 branches, subtracting nearly constant-site probability amplifies rounding: maximum error changed from `2.39e-7` to `7.99e-8`. This separate conditioning limit is documented in the fixture and tested with an explicit bound.
- At the two archived BEAST states, the largest before/after differences are `3.56e-15` per pattern, `5.69e-14` for Mk and `2.68e-12` for Mkv. Both matrix and Mk paths are included. The production scaling’s maximum per-pattern difference from the 60-digit value is `1.78e-15`; its Mkv difference from BEAST is at most `1.74e-12`.
- The 12 multistate BEAST 2.7.8 / MM 1.2.1 fixed-state fixtures still pass, as do cached-versus-full evaluation, root-change, rejected-proposal and exact checkpoint/resume tests.
- The checkpoint format is now `ROVECP03`, storing integer exponent totals. Existing executable/input/settings checks still reject incompatible restarts; checkpoints are not migrated between executables.

`numerical.json` retains old and new per-pattern values and errors. The generator and fixture are in `tools/benchmarks/scaling_reference.py` and `fixtures/validation/scaling/`.

## Reproduction

Build the baseline commit and the production checkout separately with `cargo build --release --bin rove` and `cargo bench --bench likelihood --no-run --message-format=json`. Save the CLI and the benchmark executable named in Cargo’s JSON output as `rove-before`/`likelihood-before` and `rove-subtree`/`likelihood-subtree`. In another isolated production checkout, apply `local-exponents.patch`, build with the same settings and save `rove-power-sum`/`likelihood-power-sum`. Do not mix checkpoints between variants.

With the six executables in a local directory:

```sh
python3 tools/benchmarks/scaling.py --binaries <directory> --output <new-kernel-directory> \
  --phase kernel --large-matrix <local-matrix.nex>
python3 tools/benchmarks/scaling.py --binaries <directory> --output <new-sampler-directory> \
  --phase sampler --large-matrix <local-matrix.nex>
python3 tools/benchmarks/scaling_reference.py --check
```

To reproduce fixed-state comparisons, run `tests/scaling.rs` and `tests/ascertainment.rs` against each implementation with `ROVE_SCALING_REPORT` and `ROVE_BEAST_SCALING_REPORT` pointing to new local JSON paths. Copy the new tests and scaling fixture into the baseline checkout first. The historical likelihood and sampler outputs are never used as a replacement for the independent reference.
