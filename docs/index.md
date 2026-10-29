---
layout: home
title: Phylogenetic inference in Rust
description: Rove is an ARM-first phylogenetic inference project in Rust. It starts with a likelihood kernel for morphological data, checked against a scalar reference.

hero:
  name: Rove
  text: Explore evolutionary paths.
  tagline: Phylogenetic inference in Rust, built ARM-first. Explore morphological data in a native Mac workspace.
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
    details: The Mk core prunes with the model's closed form. Each child update takes one weighted sum per pattern instead of a k×k matrix product.
    link: /guide/kernel#mk-branches
    linkText: Mk branches
  - title: Checked against a reference
    details: A plain recursive implementation stays in the crate as the reference. Property tests compare the core against it on random trees, with and without rescaling.
    link: /guide/kernel
    linkText: The likelihood core
---

<div class="home-status">

::: info Early
The likelihood core with its direct Mk path and a NEON kernel, the Mkv correction, the equality tests and a minimal standalone sampler are implemented. The relaxed-clock reference model is implemented and checked at fixed states. Posterior comparisons cover six constructed 20-taxon cases. An AVX2 path and broader cross-engine workloads remain future work. [See what exists today →](/guide/#what-exists-today)
:::

</div>

<div class="home-status">

## Built together

Rove began with conversations between Rafael Augusto and **Thiago José da Silva**, the biologist whose questions about phylogenetic analysis set the project in motion. Thiago works alongside Rafael, bringing the biological perspective, testing analyses and discussing what the results mean.

</div>
