---
description: "Measure the bundled Rove engine on your Mac and optionally share the result."
---

# Benchmark this Mac

Open **Benchmark this Mac** in the sidebar or Help menu, then choose **Run benchmark**. Rove uses its bundled engine and a synthetic matrix; no BEAST, Java, developer tools or internet connection is needed for the test. Beta access still follows its normal offline rules.

For more comparable measurements, connect power, turn off Low Power Mode and close other demanding apps. An active analysis prevents the test from starting, and a running test prevents starting or resuming an analysis. You can cancel at any time. Closing Rove stops its benchmark engine too.

## What is measured

The fixed `rove-mac-v1` protocol uses 90 taxa and 250 binary characters simulated under Mk, including constants. The model uses four gamma categories, a lognormal relaxed clock and no ascertainment filtering. Every observation starts fresh with the same seeds, runs 5,000 untimed warm-up proposals per chain, then times 50,000 proposals per chain. Tuning remains enabled.

There are three observations for **one chain**, then three for **four parallel chains**. The four-chain case performs four times as much work. Each card shows median elapsed time, total proposals per second and all three individual times, so variation remains visible. Parsing, initialization, warm-up and writing research outputs are outside the measured interval. The parallel interval includes the scheduling/join overhead of its worker threads.

This measures a fixed workload, not convergence, scientific quality or the duration of your own analysis. There is no BEAST comparison in this test. Compare matching protocol and engine fingerprints, keeping chain counts separate. Power, temperature, background work, RAM and CPU core counts can change the timings even for the same chip.

## Saved results and sharing

Each test has a separate record under `~/Library/Application Support/Rove/Benchmarks`. **Previous tests** reopens saved results; **Saved results** reveals their folder. Cancelled, interrupted or invalid tests cannot be shared. The app never uploads a test automatically.

**Share with Cortex Labs** sends the selected completed result to the project's Cloudflare D1 database. It includes:

- The six measurements, their chain counts and checksums of the synthetic final states.
- Mac model identifier, chip, physical/logical and available performance/efficiency cores, memory, macOS, architecture and Rosetta status.
- Power source, Low Power Mode and thermal category before and after the test.
- App and engine versions and executable fingerprints, backend, protocol settings and input fingerprint.
- A random ID for this test and its timestamp.

It sends no research matrix, tree, trace, file path, device serial number or beta email in the benchmark record. Sharing uses the existing beta session for authorization; the server uses expiring hashed account/IP rate-limit identifiers, separate from benchmark rows. There is no public ranking or persistent device identity. Results are self-reported, not hardware attestations.

If sending fails, the result stays on your Mac and you can retry. Sending the same record again does not create another database entry. To request removal of a shared result, provide its test ID from the local JSON file. Removing the local file alone does not remove its shared copy.

The [protocol and synthetic input](https://github.com/getcortexlabs/rove/tree/main/fixtures/benchmarks) are versioned with the engine. The CLI also exposes `rove benchmark`, emitting progress and measurements as JSON lines without writing analysis outputs.
