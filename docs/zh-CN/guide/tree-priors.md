---
description: "Rove 支持的树先验、各项假设与适用限制。"
---

# 选择树先验

树先验描述观察性状之前的分支过程。它不同于时钟 sigma 的先验（分支间速率差异），
也不同于 gamma 形状参数的先验（性状间速率差异）。

## 可用选项 · 2026 年 11 月 5 日

**Yule 已可作为同时采样物种的可选先验。** Birth–death 仍为默认模型，化石定年使用 FBD。应根据研究的采样方式与生物学假设选择模型；实现 Yule 并不能证明它更适合 Thiago 的矩阵。请参阅[已实现的模型和控件](./reference-model)。

| 分析情境 | 模型 | Rove 状态与限制 |
| --- | --- | --- |
| 视为同时采样的物种 | Birth–death | 已支持；末端高度为零、完整采样、恒定速率 |
| 同一情境，但明确假定无灭绝 | Yule | 已支持；纯出生模型，不含灭绝 |
| 有年龄信息的化石末端 | Fossilized birth–death（FBD） | 已支持；过程参数固定或估计，末端年龄可有区间；仅末端化石 |
| 可能是直接祖先的化石 | 含采样祖先的 FBD | 由 #52 单独处理 |
| 同一种群内的样本 | 溯祖 / 种群 skyline | 暂缓；不能替代物种多样化模型 |
| 不使用时钟或定年的形态分析 | 均匀拓扑与分支长度先验 | 单独评估；需要不同的树参数化 |
| 多样化或化石采集率随时间变化 | Birth–death/FBD skyline | 暂缓，需明确研究设计及足够信息以估计更多参数 |

[BEAST 的 Yule 指南](https://beast2-dev.github.io/hmc/hmc/YuleModel/index/)限定其用于同时采样的末端。
[RevBayes 形态教程](https://revbayes.github.io/tutorials/morph_tree/V2)展示无时钟方案；
其 [FBD 教程](https://revbayes.github.io/tutorials/fbd_simple/)联合建模多样化和化石采集。

缺少年龄的化石不会因此成为现生物种。把全部末端设为零是一项模型假设，不能补足缺失的年龄。
采样覆盖、灭绝和时间尺度都必须符合研究问题。矩阵大小或较高 ESS 不能决定最佳先验。

现有 birth–death 使用未缩放的 Gernhard 约定，以末端数量而非根/起源时间为条件。FBD 以起源时间为条件，不以存活或现生采样为条件，移除概率为零。固定过程与估计过程记录不同的受限密度约定；模型比较前请参阅[化石末端定年](./tip-dating)。

## Yule 实现与验证

实现采用 BEAST 2.7.8 `YuleModel` 的约定：完整采样、以末端数量为条件，
不以根或起源时间为条件，不支持化石末端或节点校准。
对于 n 个末端、物种形成率 λ 和内部高度 h，其未缩放对数密度为
`(n−1) log(λ) − λ (sum(h) + root_height)`。
这是现有 birth–death 密度在灭绝率为零时的情形，参见
[Gernhard (2008)](https://doi.org/10.1016/j.jtbi.2008.04.005)。

验证包括 BEAST 2.7.8 的 24 个固定状态、独立先验模拟、birth–death 的零灭绝极限、幂后验求积，以及检查点的完全一致恢复。该模型不包含灭绝参数及其提议。应用和 CLI 在设置、Review/History、预测及模型比较中保留这些选择。

物种形成率使用正规的 Uniform(0,U) 先验，正数上界 U 可配置。U=1000 是单位时钟高度尺度下的参考值，并非普遍适用的生物学建议。应检查仅先验运行和有依据的其他上界。边际比较要求数据、约束相同且归一化约定兼容；详见[模型说明](./reference-model)。

解释结果前，应比较独立链、仅先验运行，以及分支支持度和高度对模型假设的敏感性。
速度或支持度更高，并不说明模型在生物学上更合适。

## ORC 是另一项决策

Rove 已有显式对数正态分支速率和可选联合提议，但尚未实现整套 ORC 算子。
[XML 审计](https://github.com/getcortexlabs/rove/blob/main/fixtures/reference/20x50/orc-audit.md)
记录版本、对应关系和验证要求。#48 和 #72 评估权重期间维持当前默认方案。
[#97](https://github.com/getcortexlabs/rove/issues/97) 的拓扑与速率联合交换已实现为可选实验，仅支持未定年的松弛时钟，默认权重为零。其形态分析比较收益尚未测量；[#128](https://github.com/getcortexlabs/rove/issues/128) 记录 ORC 的部分覆盖及待完成算子和验证。
