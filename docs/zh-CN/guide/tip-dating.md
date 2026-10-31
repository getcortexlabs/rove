---
description: "使用固定年龄或年龄区间、明确的时间尺度和化石化生灭先验进行化石末端定年。"
---

# 化石末端定年

`rove sample --dating settings.json` 为形态学分析加入化石年龄和时间尺度。它支持严格或松弛时钟、字符分区、字符筛选条件、独立链和检查点。可通过 Mac 原生应用或 Rust CLI 配置。不需要安装 BEAST 或 Java。

## 在 Mac 应用中使用

1. 导入矩阵，打开 **Model**，在 **A place in time** 下启用 **Date fossils**。应用界面支持英语和葡萄牙语。
2. 选择共同时间单位，打开 **Edit tip ages**。所有末端初始设为现生（零）。搜索各个化石，选择固定年龄或从年轻到年老的年龄区间，然后点击 **Apply ages**。编辑器要求区间两端不同；相同年龄请用 **Fixed age**。
3. 为每个过程参数选择 **Keep fixed** 或 **Estimate**。填写固定值，或初值和先验。速率使用指数先验均值；现生采样使用 beta 参数 α 和 β；起源使用超过最老允许末端年龄的平均时间。这些字段初始为空。示例值用于检查软件，不是生物学推荐值。
4. 全局时钟选择 **Estimate**（初始速率和指数先验均值）或 **Keep fixed**（仅速率）。上方的演化时钟卡片仍用于选择严格或松弛分支速率。
5. 在 **Review** 检查全部设置，再开始分析。历史记录保留年龄、单位、过程参数和时钟模式；保存设置及暂停后恢复也会保留这些选择。

支持小数点、小数逗号和科学记数法，不支持千位分隔符。修改单位名称不会换算数值。关闭定年会恢复无日期模型，并保留输入值以便重新启用；导入另一矩阵会开始新设置。

注释树的比例尺显示记录的时间单位。链比较将经验证固定的参数标为 **Fixed**，因为 ESS 和 R-hat 不适用于从未采样的固定值。

## 年龄与单位

年龄**从现在向过去计量**：零表示现生分类单元，12 表示距今 12 个时间单位。示例中的 `time_unit: "Ma"` 表示百万年。所有年龄、节点高度、起源时间和分支持续时间必须使用同一单位。Rove 记录单位名称，但不进行单位换算。

时钟速率表示每个字符在每个时间单位内的期望变化数。分支的期望变化数等于持续时间 × 全局时钟速率 × 分支相对速率。松弛时钟的相对速率分布均值为一；gamma 类别进一步乘以字符速率。不启用定年时，现有预设仍使用单位时钟和年龄为零的末端。

每个分类单元都必须明确提供年龄。单个数值固定年龄；`[最小值, 最大值]` 允许在区间内估计。正年龄区间表示**标本年龄的不确定性**，不是物种首次至末次出现的地层范围。这一区别见 [RevBayes 标本教程](https://revbayes.github.io/tutorials/fbd/fbd_specimen)。区间不能包含现在；`[12,12]` 为固定年龄。名称必须与解析后的 NEXUS 标签一致，其中未加引号的下划线会转换为空格。

## 运行构造示例

```sh
cargo run --release --bin rove -- sample fixtures/validation/dating/matrix.nex \
  --dating fixtures/validation/dating/dating.json \
  --output runs/dated-example --seed 25 --steps 100000 --sample-every 100
```

此示例用于检查软件，并非生物学分析或收敛保证。其 JSON 为：

```json
{
  "time_unit": "Ma",
  "birth_rate": 0.08,
  "death_rate": 0.03,
  "fossil_sampling_rate": 0.02,
  "present_sampling": 0.7,
  "origin": 50,
  "clock_rate": 0.01,
  "clock_prior_mean": 0.01,
  "ages": {
    "Living A": 0,
    "Fossil B": [5, 9],
    "Living C": 0,
    "Fossil D": 12
  }
}
```

出生、死亡和化石采样参数是每条谱系每个时间单位的速率；present_sampling 是概率。origin 是祖先茎谱系开始的年龄，必须早于根和所有允许的末端年龄。本例中，这五个过程设置保持固定。下方可选先验允许单独估计各参数。应根据科学问题选择并进行敏感性分析；示例值不是生物学默认值。

提供 clock_prior_mean 时，clock_rate 为初值，时钟使用具有该均值的指数先验进行估计。省略 clock_prior_mean 则固定 clock_rate。`--model reference` 启用 gamma 类别和松弛时钟；`--sample-from-prior` 排除字符似然，但样本数量、年龄证据和施加的约束仍会影响所估计的过程参数。

## 固定或估计过程参数

在定年 JSON 中加入以下两个字段可估计全部五个参数。这只是片段，不是完整配置。可运行版本位于 `fixtures/validation/fbd-estimation/dating.json`：

```json
{
  "process_model": "joint_sampling_v1",
  "process_priors": {
    "birth_rate_mean": 0.08,
    "death_rate_mean": 0.03,
    "fossil_sampling_rate_mean": 0.02,
    "present_sampling_beta": [2, 2],
    "origin_excess_mean": 30
  }
}
```

省略某个参数的先验即可将其固定；`process_priors` 中至少保留一个先验。模型标识和先验对象必须同时提供。JSON 顶层的对应数值成为估计参数的初值。旧文件不包含这两个字段时，仍使用固定过程。

指数均值必须为正且有限。估计速率初值必须大于零；固定死亡速率可为零。beta 参数必须在 [0.001, 1000] 内，初始采样概率严格在零与一之间。beta 先验均值为 α / (α + β)。

`origin_excess_mean` 是**起源年龄减去最老允许末端年龄**的指数分布均值。本例中下界为 12 Ma，平均额外时间 30 Ma 对应起源先验名义均值 42 Ma。起源还必须早于当前根。该联合支持条件会被强制执行，但不会随根高度重新归一化起源先验。

速率、时间、起源和时钟可能互相补偿。全部估计可能产生宽广、相关且探索较慢的后验。应检查轨迹、独立链诊断以及对科学合理先验的敏感性；高 ESS 本身不能证明数据能够识别每个参数。

## 先验假设

FBD 包括出生、灭绝、化石采样和不完全的现生采样。树限制为**末端化石**，不含采样祖先。这是施加的约束，不是 Rove 推断出的证据。采样不会强制谱系死亡：移除概率为零。

所有过程参数固定时，限制树密度的归一化常数不变，因此被省略。估计显式选择 `joint_sampling_v1`：观察到的标本数量、年龄证据以及末端和类群约束同时为过程和树提供信息。Rove 将 FBD 采样密度、适当先验和观测因子相乘。限制空间的质量依赖参数，作为信息保留；**不能**假定它抵消，也不会按各参数值将它除去。这与为每个过程分别条件化并归一化的树先验不同。

底层 FBD 密度以起源为条件，不以存活或现生采样为条件，在验证状态上与 [BEAST 2 SA](https://www.beast2.org/xml/sa.evolution.speciation.SABirthDeathModel.html) 一致。[模型推导与验证](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/fbd-estimation) 给出联合目标的细节。仍不支持采样祖先及物种出现范围；[模型比较](./model-comparison)说明了定年边际似然何时可以比较。施加的年龄或类群约束不是字符数据的独立恢复结果。

## 输出与检查

trace 包含 fossilized_birth_death、clock_rate、dating_parameter_prior，以及按矩阵顺序排列的 tip_age_1、tip_age_2 等。启用时仍保留 gamma 和相对时钟列。固定参数没有可用 ESS；链比较将其标记为 fixed_by_model，仅在核实其固定值后将其排除于自动停止检查之外。需要估计却变成常量的参数仍会阻止自动停止。

`run.json` 保存完整设置及其指纹。检查点内嵌设置，恢复时不再需要原始 JSON。树保留明确的高度和分支持续时间；相对速率注释不包含全局时钟，后者保存在 trace 中。每个节点只带一个元数据块：严格时钟下为 `[&height=h]`，松弛时钟下非根节点为 `[&height=h,rate=r]`。BEAST 2.7.8 的 NEXUS 解析器可直接读取这些文件。早期松弛时钟运行在每个非根节点上写出 `[&height=h][&rate=r]`；Rove 仍能读取它们，将两个块合并为一个即可恢复与 BEAST 的兼容，且不改变任何数值。`rove summarize`、`inspect-tree`、`diagnose`、`compare` 和 `predict` 均支持这些输出。存在运行记录时，摘要保留时间单位名称。单独的树文件无法自行确定物理时间单位。

验证将八个固定密度与 BEAST 2.7.8 / SA 2.1.1 比较，通过解析极限和独立微分方程积分检查小树先验分布，并测试似然恢复、单位变换、检查点、导出以及不分配堆内存的提议。这些检查不证明经验化石数据上的收敛或性能。[验证材料](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/dating) 记录公式、来源、版本和复现命令。

估计任一过程参数时，trace 还记录 `fbd_birth_rate`、`fbd_death_rate`、`fbd_fossil_sampling_rate`、`fbd_present_sampling` 和 `fbd_origin`。其中固定的成员也会记录，并在链比较中验证。`run.json` 保存 `dating_density_convention` 和完整先验。原生设置中，过程估计使用格式 4，固定过程定年使用格式 3，无日期分析使用格式 2；旧应用拒绝不支持的格式。检查点恢复需要相同的引擎可执行文件。

估计验证新增八个完整先验状态与 BEAST 2.7.8 / SA 2.1.1 的比较、解析条件分布、独立联合积分、固定与估计混合运行及精确恢复测试。[构造数据报告](https://github.com/getcortexlabs/rove/tree/main/benchmarks/fbd-estimation/2026-10-30) 记录时间、ESS、R-hat、类群差异和相关性。它不证明经验化石数据上的准确性、可识别性或性能。

[Gamma 频率研究](./reference-model#gamma-提案频率)发现，更新频率过低限制了此构造示例的采样。已评估的 CLI 调整可以提高 gamma ESS，但未满足成为新默认设置的全部效率标准。原生应用仍使用现有设置。
