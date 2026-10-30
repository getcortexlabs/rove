---
description: "使用固定年龄或年龄区间、明确的时间尺度和化石化生灭先验进行化石末端定年。"
---

# 化石末端定年

`rove sample --dating settings.json` 为形态学分析加入化石年龄和时间尺度。它支持严格或松弛时钟、字符分区、字符筛选条件、独立链和检查点。目前通过 Rust CLI 配置；Mac 设置界面还没有定年控件。不需要安装 BEAST 或 Java。

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

出生、死亡和化石采样参数是每条谱系每个时间单位的速率；present_sampling 是概率。origin 是祖先茎谱系开始的年龄，必须早于根和所有允许的末端年龄。这五个过程设置保持固定。应根据科学问题选择并进行敏感性分析；示例值不是生物学默认值。

提供 clock_prior_mean 时，clock_rate 为初值，时钟使用具有该均值的指数先验进行估计。省略 clock_prior_mean 则固定 clock_rate。`--model reference` 启用 gamma 类别和松弛时钟；`--sample-from-prior` 在不使用字符似然的情况下检查联合先验。

## 先验假设

FBD 过程包括出生、灭绝、化石采样及不完全的现生采样。本实现将树限制为**末端化石**，不包含采样祖先。采集到化石并不强制该谱系死亡：模型中的移除概率为零。起源和过程参数固定，因此限制后的树空间归一化常数在采样过程中不变。

在已验证的状态上，FBD 对数密度与 [BEAST 2 SA](https://www.beast2.org/xml/sa.evolution.speciation.SABirthDeathModel.html) 的固定起源、未以存活为条件的约定一致。它是**未归一化的限制密度**，不能用于边际似然或贝叶斯因子。本版本不估计过程参数或起源，不采样祖先，也不建模物种出现范围。年龄区间是分析约束；恢复该区间不构成独立的化石年龄证据。

## 输出与检查

trace 包含 fossilized_birth_death、clock_rate、dating_parameter_prior，以及按矩阵顺序排列的 tip_age_1、tip_age_2 等。启用时仍保留 gamma 和相对时钟列。固定参数没有可用 ESS；链比较将其标记为 fixed_by_model，仅在核实其固定值后将其排除于自动停止检查之外。需要估计却变成常量的参数仍会阻止自动停止。

`run.json` 保存完整设置及其指纹。检查点内嵌设置，恢复时不再需要原始 JSON。树保留明确的高度和分支持续时间；相对速率注释不包含全局时钟，后者保存在 trace 中。`rove summarize`、`inspect-tree`、`diagnose`、`compare` 和 `predict` 均支持这些输出。存在运行记录时，摘要保留时间单位名称。单独的树文件无法自行确定物理时间单位。

验证将八个固定密度与 BEAST 2.7.8 / SA 2.1.1 比较，通过解析极限和独立微分方程积分检查小树先验分布，并测试似然恢复、单位变换、检查点、导出以及不分配堆内存的提议。这些检查不证明经验化石数据上的收敛或性能。[验证材料](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/dating) 记录公式、来源、版本和复现命令。
