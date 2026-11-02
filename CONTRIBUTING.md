# Contributing

Rove's native app is a complete interface to its analysis engine. A feature is
not finished when it only works from the terminal. Changes to CLI commands or
scientific options must include their Swift controls, validation, persistence,
review/history presentation, and native execution tests in the same PR.

Keep the [capability map](tools/native/cli-parity.json) current. The CI check
catches newly introduced flags; reviewers must also check their values and
behavior. Protocol flags such as progress events, fingerprints and cancellation
are wired automatically, rather than exposed as user settings. Performance
harnesses under `examples/`, `benches/` and `tools/` are development tools, not
analysis features.

Code, comments, issues, PRs and commit messages use English. Native product text
ships in English and Brazilian Portuguese; update all four documentation
languages together. Keep commits focused and omit coauthor trailers.

Describe the resulting behavior, relevant numerical assumptions and what was
tested. Use measured evidence for performance claims. Do not change scientific
defaults on the strength of a single benchmark. Private research notes and
local datasets stay out of commits.

Use a branch and open a PR against `main`. Run checks appropriate to the change;
CI also checks Rust, native execution, documentation and analysis tools. Release
packaging and notifications are described in [scripts/RELEASES.md](scripts/RELEASES.md).
