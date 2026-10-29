# Hand-checkable posterior trees

`four-taxa.trees` contains four rooted clock trees, with all terminals at time
zero. With one discarded sample, AB and CD each occur in two of three trees.
The first retained tree wins MCC; AB's conditional mean height is 1.5, its
mean MRCA height across all retained trees is 8/3, and mean root height is 4.
Without burn-in, AB|CD and AC|BD tie, and the first sampled AC|BD tree wins.
These are constructed post-processing examples, not MCMC output.
