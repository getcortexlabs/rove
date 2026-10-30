---
description: "Rove 的实现顺序：先确立正确性和经过测量的基线，再优化内核并构建独立推断。"
---

# 路线图

Rove 处于早期。没有发布版本，没有发布的 crate，也没有日期。这是实现顺序，不是时间表。

| 步骤 | 状态 |
| --- | --- |
| 标量似然核心与数值参考测试 | 已完成 |
| ARM64、x86_64 和最低 Rust 版本上的 CI | 已完成 |
| 可复现的 BEAST 2.7.8 参考分析 | 已完成 |
| 形态学 NEXUS 与观测模式压缩 | 已完成 |
| Rust 内核基准测试基线 | 已完成，#8 |
| 完整推断和跨引擎基准测试 | 已测量 20/90/500 分类单元运行及内核适配器，并记录采样局限，#8 |
| Mkv 确认偏差校正 | 已完成 |
| 直接 Mk 剪枝 | 已完成 |
| 最小的独立 MCMC | 已完成 |
| NEON 向量化 | 已完成 |
| 松弛时钟参考模型 | 已完成，#16 |
| 完整推断验证 | 已完成六个 20 分类单元案例，#12 |
| 松弛时钟采样效率 | 已评估；联合移动可选，保留经典默认调度，#27 |
| 原生 Mac 导入与设置预览版 | 已实现，#28 |
| 应用内运行与进度 | 已实现，#29 |
| 运行历史与配置复用 | 已实现，#30 |
| 明确选择外群及核对已应用约束 | 已实现，#31 |
| 后验 MCC 树汇总与注释导出 | 已实现，#32 |
| 原生注释树查看器与 SVG 图形 | 已实现，#33 |
| 原生采样诊断与 burn-in 检查 | 已实现，#34 |
| 并行链与收敛报告 | 已实现，#23 |
| 二次幂缩放 | 已实现，#18 |
| 检查点与恢复运行 | 已在 CLI 和原生 History 中实现，#22 |
| 按状态数划分分区 | 已实现，#19 |
| 以简约信息性状为条件 | 已实现，#20 |
| 有序性状 | 计划中，#21 |
| 用于模型比较的边际似然 | 计划中，#24 |
| 带末端日期的化石化生灭先验 | 计划中，#25 |
| 后验预测检验 | 计划中，#26 |
| 子树内重复的评估 | 已评估；实验性功能，默认关闭，#17 |
| 可选的 BEAST 2 JNI 与 BEAGLE C ABI 适配器 | 可能，#6 和 #7 |

## 建立基线

[核心](./kernel)已与一个独立的标量实现核对。[NEXUS 读取器](./nexus)在不依赖 BEAST 的情况下提供带权重的观测。[参考实验](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md)记录了确切的输入、模型设置、两次完成的 BEAST 运行和固定的数值状态。

[Apple M4 Max 基线](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-26/README.md)记录了 11 个 Rust 工作负载，分别测量初始化、整树计算、局部更新和提议状态管理。报告包含原始样本、内存分配观测以及三个独立进程之间的变动。[完整模型比较](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md)现已记录六个 20 分类单元案例的运行时间和每秒 ESS；[扩展报告](https://github.com/getcortexlabs/rove/blob/main/benchmarks/expanded/2026-10-30/README.md)增加了 90 和 500 个分类单元的运行及 BEAST/BEAGLE 内核比较，并记录收敛局限。下文的严格时钟比较记录了两个引擎在一个小型配置上的运行时间，但它不是受控的基准测试。

## 先正确，再优化

Mkv 把似然条件化于仅含可变性状的矩阵的观测过程。它重现了两个存档的 BEAST 状态，其恒定模式与观测到的模式权重保持分开。

直接 Mk 路径和 NEON 后端已实现，测量结果见 [Mk 报告](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md)和 [NEON 报告](https://github.com/getcortexlabs/rove/blob/main/benchmarks/neon/2026-10-28/README.md)。二次幂重缩放消除了逐节点对数计算，并缓存各子树的指数总和。[对比报告](https://github.com/getcortexlabs/rove/blob/main/benchmarks/scaling/2026-10-30/README.md)在两种后端上分别测量这两项改动，包括采样器运行及变慢的用例。可移植实现仍是参考和回退路径。

## 独立推断

最初的[最小采样器](./sampler)支持二态 Mk 或 Mkv、严格时钟和生灭先验。在与 BEAST 2.7.8 使用相同移动和固定步长的情况下，它在参考矩阵上的后验均值和支系频率满足了运行前确定的标准（[报告](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md)）。[参考模型](./reference-model)已包含 gamma 类别和对数正态松弛时钟，逐项重现了两个归档状态。六个含 20 个分类单元的构造案例中，后验汇总和支系频率满足了运行前确定的标准（[报告](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md)）。验证从固定状态的似然推进到已知矩阵上的完整运行。采样质量，包括单位时间的 ESS 和树诊断，与运行时间一同计入。

提议评估（#27）已记录，计划中的功能按形态学分析的需要排列：有序性状（#21）、用于模型比较的边际似然（#24）、带末端日期的化石化生灭先验（#25）以及后验预测检验（#26）。RevBayes 为形态学数据记录了其中每一项；在 BEAST 没有对应功能时，它可以作为第二个参考。

[原生 Mac 预览版](./macos-preview)已能导入和显示矩阵并准备分析设置。现在可运行内置 Rust 引擎，显示实际进度、取消运行，并在验证输入指纹后重新打开配置。BEAST 和 BEAGLE 适配器是可选的兼容路径，不是 Rust 引擎的依赖。

关注[代码仓库的 issues](https://github.com/getcortexlabs/rove/issues) 了解验收标准和结果。

状态数分区（#19）已在引擎、CLI、原生应用和检查点中实现。模型规则和固定状态验证见 [NEXUS 指南](./nexus#按状态数划分分区)。

简约信息性状条件化（#20）已在引擎、CLI、原生应用和检查点中实现。输入规则与验证见[性状选择](./character-selection)。
