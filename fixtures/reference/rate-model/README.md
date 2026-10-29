# Rate-model numerical reference

`beast-2.7.8.json` records 20 four-category gamma mixtures over the full shape
support, and log densities of the reference clock priors at six standard
deviations and six branch rates. It also records the effective gamma bounds
and the clock's rate mapping with the root at either internal node of a
three-tip tree. Clock initialization overrides the XML's rate lower bound
of 1e-100 with zero; the export records that change. The provenance records Java, the exporter
source hash and every JAR hash. These are calculations through BEAST's APIs,
not values computed by Rove.

Reproduce with the packages of the [20 × 50 reference](../20x50/README.md):

```sh
python3 tools/beast/export_rate_model.py --packages /path/to/packages \
  --launcher /path/to/launcher.jar --output /path/to/new-rate-model.json
```

`tools/benchmarks/prepare.py` generates `tests/support/rate_model.rs` from
this file. Tests also use closed-form exponential and Erlang probabilities,
independently of BEAST.

Both programs use the gamma quantiles at 1/8, 3/8, 5/8 and 7/8, each weighted
1/4, then normalize the rates to mean one. Rove inverts the gamma CDF using a
positive series and bracketed Newton steps in log space; BEAST 2.7.8 uses its
legacy AS91 approximation. The largest absolute category-rate difference on
this grid is 1.39e-8. This numerical difference is distinct from a change in
the discretization or the probability model.
