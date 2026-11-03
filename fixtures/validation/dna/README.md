# Synthetic aligned DNA

All sequences here are synthetic and redistributable. The `matrix` pair has
identical observations in FASTA and NEXUS, including constant sites, R/Y/N
ambiguity, missing cells and a gap. The `complete` pair uses only A/C/G/T and
can also exercise posterior prediction. FASTA descriptions are ignored and
underscores in identifiers follow Rove's existing NEXUS-to-space convention.

`DnaStates.java` exports six fixed states from BEAST.base 2.7.8 with
`java.only=true`, `scaling=always`, `useAmbiguities=true`, strict clock rate 1,
height increments 0.001/0.05/1.7, and one/four gamma categories (shape 0.7).
The TSV includes the actual category rates. Jar fingerprints are recorded in
`provenance.json`. The engine does not depend on BEAST or Java.

Reproduce using a local pinned BEAST package directory and launcher:

```sh
javac --release 17 -cp /path/BEAST.base.jar:/path/launcher.jar -d /tmp/rove-dna tools/beast/DnaStates.java
java -Dbeast.user.package.dir=/path/packages -cp /tmp/rove-dna:/path/BEAST.base.jar:/path/launcher.jar DnaStates /tmp/dna.tsv
```

`tests/dna.rs` independently enumerates internal states with JC69's closed-form
transition probabilities. Fixed-rate kernel comparisons use absolute 1e-10
log-likelihood tolerance. Shape-based comparisons allow 1e-7 because BEAST's
AS91 gamma quantiles have lower precision (already tested in `tests/rates.rs`);
these category rates differ by less than 3e-8. Tests also cover IUPAC masks,
constant sites, gaps, model identity, rejection of unsupported options,
simulation frequencies, cache correctness, and exact checkpoint continuation.
No benchmark or mixing comparison is included.
