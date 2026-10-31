---
description: "基于幂后验，用阶石法和路径采样估计对数边际似然，并通过贝叶斯因子比较模型。"
---

# 模型比较

`rove marginal` 估计所配置模型的对数边际似然：在模型下对所有树和参数积分后得到的矩阵概率。两个模型的对数边际似然之差即对数贝叶斯因子。估计来自幂后验：即把似然取某个指数次幂后的链目标，指数为零时是先验，为一时是后验。该方法沿用 [RevBayes 模型选择教程](https://revbayes.github.io/tutorials/model_selection_bayes_factors/bf_intro) 和 BEAST 2 MODEL_SELECTION 包的 PathSampler。运行它不需要任何外部软件。

该命令在 Rust CLI 中可用。Mac 应用未提供它。

## 运行一次估计

```sh
cargo run --release --bin rove -- marginal fixtures/reference/20x50/matrix.nex \
  --output runs/marginal-mkv --seed 1 --steps 1000000 --sample-every 1000 \
  --powers 32 --alpha 0.3 --burnin-percent 20 \
  --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
```

`rove sample` 的模型选项同样适用：`--model`、`--conditioning`、`--partitioning`、`--gaps`、`--dating`、`--monophyly`、`--weight`、`--tuning`、`--no-tuning` 和 `--backend`。要比较模型，请在同一矩阵、同一约束下对每个模型各运行一次，然后将估计值相减。`--sample-from-prior`、并行链、检查点和停止诊断不可用。

`--powers` 是似然指数的个数，包含零和一。它们是 Beta(α, 1) 分布在等距位置上的分位数，`β_i = (i / (powers − 1))^(1/α)`；较小的 `--alpha` 会把更多指数放在零附近，那里平均对数似然变化最快。上述取值沿用 BEAST 2 PathSampler 的约定。每个幂运行 `--steps` 次提议，每 `--sample-every` 步记录一次；每个幂的样本按数量丢弃前 `--burnin-percent`。链从幂一运行到零，每个幂从前一个幂的最终状态继续，并保留提议的调节。

## 输出

输出目录包含 `run.json`（模型、幂序列和算子计数）、`marginal.json`（估计值），以及每个幂一个 `power-NN/trace.log`。这些 trace 带有普通 trace 中的后验、似然和先验列，未乘以指数；指数只进入接受率。不写出树。

`marginal.json` 报告两个带标准误的估计：

- **阶石法**（stepping stone，[Xie 等 2011](https://doi.org/10.1093/sysbio/syq085)）对相邻幂求和：在较低的幂下取 `L^(β_k − β_{k−1})` 均值的对数。其标准误按 delta 方法累加每一项的方差，使用该项权重的有效样本量。
- **路径采样**（path sampling，[Lartillot 与 Philippe 2006](https://doi.org/10.1080/10635150500433722)）用梯形法则对平均对数似然沿指数积分。其标准误累加每个均值由有效样本量得到的方差。梯形法则自身的误差不计入，因此当幂序列较粗时，两个估计之差可能超过它们的标准误。

对每个幂，文件记录保留的样本数、对数似然的均值与标准差、其 ESS，以及是否达到自相关的滞后上限。幂零附近 ESS 偏低很常见：那些链在探索先验，树自由移动，似然变化很大。每个幂更多的步数或更多的幂可以降低误差。

## 如何解读结果

依照 [Kass 与 Raftery（1995）](https://doi.org/10.1080/01621459.1995.10476572)，对数贝叶斯因子超过约 3 到 5 通常读作对第一个模型的强支持。只比较同一矩阵、同一分类单元、同一约束下的估计；在解读小于几个标准误的差异之前，请用其他种子重复估计。边际似然依赖于先验，包括生灭先验和时钟先验，因此贝叶斯因子比较的是模型连同其先验。

Mk 和 Mkv 为不同的观测过程定义似然：Mkv 以每个性状都可变为条件。两者之间的贝叶斯因子既比较模型，也比较这些过程；参见[性状选择](./character-selection)。

在固定过程的化石末端定年下，FBD 密度省略了一个依赖于过程设置、末端和年龄界限的归一化常数。只有当这些设置完全相同时才比较定年模型，这样常数才会抵消。在估计过程参数时，联合采样目标包含观测因子，其归一化尚未为模型比较推导；此类比较不受支持。参见[化石末端定年](./tip-dating)。

## 验证

测试在带精确样本的共轭高斯模型上检验两个估计量，包括四十次重复中标准误的校准；并与三个分类单元、一个自由节点高度的树上的数值积分比较，该积分由引擎自身的密度算出。幂为零的链逐步复现采样先验的链，同时仍计算似然；[0, 1] 之外的指数被拒绝，检查点记录当前使用的指数。[与 BEAST 2.7.8 的比较](https://github.com/getcortexlabs/rove/blob/main/benchmarks/marginal-likelihood/2026-10-31/README.md)在所提供矩阵的严格时钟 Mkv 和 Mk 配置上运行了 MODEL_SELECTION 1.6.2 的 PathSampler 和 `rove marginal`，32 个幂，每个幂 1,000,000 步，每个程序三个种子。每一对 Rove–BEAST 阶石法估计都在 2.6 个合并标准误内一致，标准误在 0.20 到 0.35 对数单位之间，路径采样也同样一致；两个程序的估计均由同一实现从各自的日志重新计算。这确立的是同一估计量的不同实现在一个小矩阵上的一致性，而不是在经验数据上的模型选择表现。
