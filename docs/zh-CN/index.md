---
layout: home
title: 用 Rust 实现的系统发育推断
description: Rove 是一个以 ARM 为先、用 Rust 编写的系统发育推断项目。它从一个面向形态学数据的似然内核起步，并与标量参考实现进行核对。

hero:
  name: Rove
  text: 探索演化的路径。
  tagline: 用 Rust 编写、以 ARM 为先的系统发育推断。今天，是面向形态学数据的似然内核。下一步，是独立分析。
  image:
    src: /images/rove-logo.webp
    width: 640
    height: 640
    fetchpriority: high
    decoding: async
    alt: Rove 的标志，由相连节点构成的字母 R。
  actions:
    - theme: brand
      text: Rove 是什么？
      link: /zh-CN/guide/
    - theme: alt
      text: 似然核心
      link: /zh-CN/guide/kernel
    - theme: alt
      text: GitHub
      link: https://github.com/getcortexlabs/rove

features:
  - title: 为数据而设计的内核
    details: 形态学矩阵大多由二态和三态性状组成。Rove 按位点模式主序存放部分似然，因此无论状态数是多少，内层循环都沿位点模式运行。
    link: /zh-CN/guide/design
    linkText: 设计
  - title: Mk 的闭式解
    details: Mk 核心直接用模型的闭式解做剪枝。每次子节点更新只需对每个模式做一次加权求和，而不是 k×k 矩阵乘积。
    link: /zh-CN/guide/kernel#mk-分枝
    linkText: Mk 分枝
  - title: 与参考实现核对
    details: 一个朴素的递归实现作为参考保留在 crate 中。属性测试在随机树上把核心与它比较，有重缩放和无重缩放两种情况都测。
    link: /zh-CN/guide/kernel
    linkText: 似然核心
---

<div class="home-status">

::: info 早期
带直接 Mk 路径的似然核心、Mkv 校正、等价性测试和一个最小的独立采样器已经实现。松弛时钟模型、SIMD 路径和受控的跨引擎基准测试尚未实现。[看看今天已有什么 →](/zh-CN/guide/#今天已有什么)
:::

</div>
