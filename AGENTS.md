# Working on Rove

## Project direction

Rove is an independent phylogenetic inference project with a Rust engine,
focused first on morphological data and ARM CPUs. The intended product is a
native macOS application distributed as a DMG. SwiftUI and AppKit are the
proposed interface tools; application UI work requires an explicit request.

BEAST is a validation and benchmarking reference. Optional BEAST or BEAGLE
adapters must not become dependencies of the standalone Rust engine.
Distinguish BEAST 2 from BEAST X in issues, experiments, and results.

The documentation site under `docs/` is published in English, Brazilian
Portuguese, Spanish and Simplified Chinese. English is the source; the
`pt-BR`, `es` and `zh-CN` trees are translations of it and change together
with it. Keep public documentation proportional to implemented features; do
not document planned behavior as available.

## Language and writing

- Write code, identifiers, comments, public documentation, issues, PRs, commit
  messages, and release notes in English.
- Outside the documentation site's translated trees, use Portuguese in an
  artifact only when the user explicitly requests it. The two private
  learning documents already have that exception. Reply to the user in the
  language of the conversation.
- Write directly and specifically. Describe the actual problem, change, and
  evidence. Avoid promotional claims, stock introductions, and filler.
- Use short, imperative commit subjects. Add a body when context is needed.
- PR descriptions should explain the problem, resulting behavior, validation,
  and any relevant limitations. Do not include a transcript of the work.
- Never add `Co-authored-by` trailers or automatic assistant signatures to
  commits, PRs, issues, or source files. Use the configured Git author without
  inventing contributor identities.

## Code

- Prefer small, idiomatic Rust functions, clear ownership, and concrete names.
  Add an abstraction when it removes real duplication or serves an existing
  requirement. Avoid speculative frameworks and unnecessary dependencies.
- Comments should explain a mathematical assumption, invariant, or non-obvious
  decision. Do not narrate code that already explains itself.
- Prefer safe Rust. Introduce unsafe code only with a concrete reason,
  documented safety invariants, and appropriate validation.
- Keep a scalar reference for optimized numerical paths. Preserve numerical
  correctness, scaling, and state restoration when changing layout or SIMD.
- Measure allocations and data movement in hot paths. Use profiles and
  benchmarks to decide whether an optimization is worthwhile.
- Run checks appropriate to the change. For Rust changes, use
  `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test`. For documentation tooling changes, use `npm run docs:build`.
- Add tests for meaningful behavior and regressions. Do not add tests that
  merely repeat the implementation or test static prose.

## Scientific claims and performance

- Separate implemented behavior, proposals, and measured results.
- Record data provenance, program and package versions, model settings,
  hardware, build settings, and commands with benchmark results.
- Distinguish observed characters from compressed site patterns.
- Compare likelihoods at fixed states before comparing complete inference
  runs. Matching seeds do not guarantee identical MCMC trajectories across
  implementations.
- For inference, report sampling quality as well as runtime, including ESS
  per time and tree diagnostics where applicable. Posterior support imposed
  by a monophyly constraint is not independent recovery from the data.
- Do not promise speedups from the choice of language or architecture alone.
  Results for one BEAST implementation do not establish results for another.

## Repository and private notes

- The canonical repository is `https://github.com/getcortexlabs/rove`.
- Check existing issues before creating new ones. Keep scope, dependencies,
  and completion criteria current as the implementation changes.
- `DIARIO-LOCAL.md` and `GUIA-DO-ZERO-LOCAL.md` are private, local-only notes.
  Keep them excluded through `.git/info/exclude`. Never stage, commit,
  upload, or quote their contents in public artifacts.
- Keep VitePress rooted in `docs/`. The private notes must never enter its
  generated output.
- Review the staged file list before committing. Keep build output, caches,
  local experiment logs, and unrelated changes out of commits.
