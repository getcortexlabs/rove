---
layout: home
title: Phylogenetic inference in Rust
description: Rove is an ARM-first phylogenetic inference project in Rust. It starts with a likelihood kernel for morphological data, checked against a scalar reference.

hero:
  name: Rove
  text: Explore evolutionary paths.
  tagline: Phylogenetic inference in Rust, built ARM-first. Today, a likelihood kernel for morphological data. Next, standalone analysis.
  image:
    src: /images/rove-logo.webp
    width: 640
    height: 640
    fetchpriority: high
    decoding: async
    alt: The Rove logo, a letter R drawn as connected nodes.
  actions:
    - theme: brand
      text: What is Rove?
      link: /guide/
    - theme: alt
      text: The likelihood core
      link: /guide/kernel
    - theme: alt
      text: GitHub
      link: https://github.com/getcortexlabs/rove

features:
  - title: A kernel shaped for the data
    details: Morphological matrices are mostly binary and ternary characters. Rove lays partials out pattern-major, so the inner loop runs across site patterns whatever the number of states.
    link: /guide/design
    linkText: The design
  - title: Mk, in closed form
    details: The Mk model ships with its closed-form transition matrix. A fast path that skips the generic k×k product is the next step on the roadmap.
    link: /guide/roadmap
    linkText: The roadmap
  - title: Checked against a reference
    details: A plain recursive implementation stays in the crate as the reference. Property tests compare the core against it on random trees, with and without rescaling.
    link: /guide/kernel
    linkText: The likelihood core
---

<div class="home-status">

::: info Early
The likelihood core, the Mk model and the equality tests are implemented. Standalone inference, SIMD paths and benchmarks are not. [See what exists today →](/guide/#what-exists-today)
:::

</div>
