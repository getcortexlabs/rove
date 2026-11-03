# Yule fixed-state reference

`beast-2.7.8.tsv` contains 24 synthetic ladder trees: 2/3/5/8 tips, birth
rates 0.02/1.3/200, and successive node-height increments 0.001/0.7.
`provenance.json` pins the BEAST.base jar fingerprint and Java version.
No private matrix is used. No BEAST code is required at engine runtime.

To regenerate with the pinned jar:

```sh
javac --release 17 -cp /path/to/BEAST.base.jar -d /tmp/rove-yule tools/beast/YuleStates.java
java -cp /tmp/rove-yule:/path/to/BEAST.base.jar YuleStates > /tmp/yule.tsv
```

`tests/yule.rs` compares all three logged densities, derives the labeled-tree
normalization coefficient independently, and tests MCMC against a separate
waiting-time/pair-merger simulator, with and without monophyly. It also checks
the proper birth-rate prior. These are correctness checks, not performance
measurements. See the reference-model guide for model scope and constants.
