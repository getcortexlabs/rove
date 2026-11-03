# Combined morphology and DNA reference

Synthetic five-taxon, six-character binary morphology plus the twelve-site
`../dna/complete.fasta` alignment. `study.mixed.json` embeds their original UTF-8
contents and SHA-256 digests; no private matrices are used.

`beast-2.7.8.tsv` contains 36 states from BEAST.base 2.7.8 + MM 1.2.1:
ladder internal heights 1–4 times `height_step`, contemporaneous tips, strict
branch clock, morphology/DNA rate ratios 0.2/1/3.5, 1/4 gamma categories with
shape 0.7, and unconditioned/variable/informative morphology (codes 0/1/2).
DNA is always unconditioned JC69. Columns list both likelihood components and
their sum. The Java oracle enumerates excluded binary columns for ascertainment;
it does not use Rove's likelihood or informative-probability implementation.

Reproduce with `tools/beast/MixedStates.java`, Java 17, the pinned BEAST.base jar,
MM addon jar and `launcher.jar` on the classpath. Set `beast.user.package.dir` to
the corresponding package directory; pass the destination TSV as the only
argument. `provenance.json` records versions and SHA-256 hashes. This is a
fixed-state reference, not a runtime dependency or performance measurement.

`cargo test --test mixed` checks the reference at tolerance 1e-7 (including gamma
quantile approximation differences), independent component sums, a normalized
lognormal rate prior and stationary simulation, taxon matching and missing
masks, cache recomputation and exact state replay. CLI/native tests additionally
exercise combined-source creation, settings/history, resume, predictive checks,
marginal estimates and explicit original-input export consent.
