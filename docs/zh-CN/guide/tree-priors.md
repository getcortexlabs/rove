---
description: "Rove 支持的树先验，以及参考 BEAST 和 RevBayes 后选定的下一项扩展。"
---

# 选择树先验

树先验描述观察性状之前的分支过程。它不同于时钟 sigma 的先验（分支间速率差异），
也不同于 gamma 形状参数的先验（性状间速率差异）。

## 决策 · 2026 年 11 月 2 日

保留当前默认 birth–death 模型和 FBD 定年。选择 **Yule 作为下一项针对同时采样物种的可选先验**，
不替换默认值。这是开发方向，不是 Yule 更适合 Thiago 数据的证据。Rove 尚未实现 Yule，实现工作记录在 [#96](https://github.com/getcortexlabs/rove/issues/96)。

| 分析情境 | 模型 | Rove 状态与限制 |
| --- | --- | --- |
| 视为同时采样的物种 | Birth–death | 已支持；末端高度为零、完整采样、恒定速率 |
| 同一情境，但明确假定无灭绝 | Yule | 下一项扩展；用于比较的受限模型 |
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

## Yule 实现必须通过的验证

第一版采用 BEAST 2.7.8 `YuleModel` 的约定：完整采样、以末端数量为条件，
不以根或起源时间为条件，不支持化石末端或节点校准。
对于 n 个末端、物种形成率 λ 和内部高度 h，其未缩放对数密度为
`(n−1) log(λ) − λ (sum(h) + root_height)`。
这是现有 birth–death 密度在灭绝率为零时的情形，参见
[Gernhard (2008)](https://doi.org/10.1016/j.jtbi.2008.04.005)。

实现需包含与固定版本 BEAST 的密度对照、独立先验模拟、移除灭绝参数及其提议、
明确的物种形成率先验、版本化设置、检查点、输出记录及英/葡双语控件。
单位时钟下保留 Uniform(0,1000) 作为对照先验，注明尺度并允许有意识地配置。
允许边际似然比较之前，必须核对密度常数及约束。

解释结果前，应比较独立链、仅先验运行，以及分支支持度和高度对模型假设的敏感性。
速度或支持度更高，并不说明模型在生物学上更合适。

## ORC 是另一项决策

Rove 已有显式对数正态分支速率和可选联合提议，但尚未实现整套 ORC 算子。
[XML 审计](https://github.com/getcortexlabs/rove/blob/main/fixtures/reference/20x50/orc-audit.md)
记录版本、对应关系和验证要求。#48 和 #72 评估权重期间维持当前默认方案。
下一项独立算子实验是 [NER #97](https://github.com/getcortexlabs/rove/issues/97)：同时改变邻近分支与速率。它对形态数据的收益仍需测量。
