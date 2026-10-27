---
description: "Rove 似然核心今天计算什么，以及如何从 Rust 调用它：末端节点、转移矩阵、部分似然、Mkv 校正、重缩放和对数似然。"
---

# 似然核心

`LikelihoodCore` 在给定每条分枝、每个速率类别各一个转移矩阵的前提下，为一个位点模式矩阵计算有根二叉树的对数似然。本页描述代码仓库中现有的 API。crates.io 上还没有 crate；要试用，请把代码仓库作为 Git 依赖使用。

::: info 范围
核心接收转移矩阵和末端节点数据，返回似然。独立的 [NEXUS 读取器](./nexus)负责准备观测和模式权重，[Mkv 校正](#以可变性状为条件)则把似然条件化于可变性状。树提议和 gamma 速率的计算仍是未来的工作。见[路线图](./roadmap)。
:::

## 节点与缓冲区

节点从 `0` 编号到 `node_count − 1`。核心不知道哪些节点是末端节点；它只要求在每个内部节点的两个子节点就绪之后，为该节点调用一次 `calculate_partials`。

每个节点拥有两个部分似然缓冲区和两个转移矩阵缓冲区。在一次提议中覆盖某个节点之前，先调用 `set_node_partials_for_update` 或 `set_node_matrix_for_update`，并用 `store`、`restore` 和 `unstore` 管理哪一组是当前的。[设计](./design#双缓冲与重缩放)页解释这一机制。

## 末端节点

`set_tip_states(node, states)` 接收每个模式一个状态索引。等于或大于状态数的值表示缺失或不确定的数据：每个状态的部分似然都取 1.0。

`set_tip_partials(node, partials)` 接收 `patterns × states` 个值，按位点模式主序排列，用于带不确定性的末端节点。

两者都写入该节点的两个缓冲区，因此末端节点在 `restore` 之后依然保留。

## 转移矩阵

`set_node_matrix(node, category, matrix)` 为 `node` 上方的分枝、在一个速率类别中，存放一个 `states × states` 的行主序矩阵。`model::Mk` 构建这些矩阵：

```rust
use rove::model::Mk;

let model = Mk::uniform(2);
let mut matrix = vec![0.0; 4];
model.transition_matrix(0.1, &mut matrix);
```

`Mk::uniform(k)` 使用相等的频率；`Mk::with_frequencies` 接收你自己的频率。`transition_matrix(distance, out)` 为一条分枝填充 `out`，分枝长度以每个性状的期望变化次数给出。速率异质性由调用方处理：为每个类别计算一个矩阵，枝长乘以该类别的速率。

## 计算似然

一个二态性状的四个模式，分布在三个末端节点上，树为 `((A, B), C)`：

```rust
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling};

// 节点：A = 0，B = 1，C = 2，(A, B) = 3，根 = 4。
let states = 2;
let patterns = 4;
let categories = 1;
let nodes = 5;

let mut core = LikelihoodCore::new(states, patterns, categories, nodes, Scaling::Always);
core.set_tip_states(0, &[0, 0, 1, 1]);
core.set_tip_states(1, &[0, 1, 1, 0]);
core.set_tip_states(2, &[0, 1, 0, 2]); // 2 超出范围：缺失数据

let model = Mk::uniform(states);
let mut matrix = vec![0.0; states * states];
for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    model.transition_matrix(length, &mut matrix);
    core.set_node_matrix(node, 0, &matrix);
}

core.calculate_partials(0, 1, 3);
core.calculate_partials(3, 2, 4);

let pattern_weights = [1.0, 1.0, 1.0, 1.0];
let log_likelihood = core.log_likelihood(4, &[1.0], model.frequencies(), &pattern_weights);
assert!((log_likelihood - -8.957188856221643).abs() < 1e-12);
```

`log_likelihood` 按类别权重对根节点的部分似然做积分，乘以状态频率，逐模式取对数，加上所有缩放因子，再按模式权重求和。这些步骤也可以分别通过 `integrate_partials`、`calculate_log_likelihoods` 和 `pattern_log_likelihoods` 单独调用。

参考实现对这棵树给出相同的值。这就是等价性测试在随机输入上所检查的。

## 以可变性状为条件

形态学矩阵通常只保留在分类单元之间有变化的性状。它们的似然必须以这种选择为条件，如 Lewis 的 [Mkv 模型](https://doi.org/10.1080/106351501753462876)。设 `Pₛ` 为所有分类单元都处于状态 `s` 的概率，并已按速率类别混合；似然为 `Lᵢ`、权重为 `wᵢ` 的每个观测模式 `i` 贡献

```text
wᵢ · (ln Lᵢ − ln(1 − Σₛ Pₛ))
```

`ascertainment::VariableCharacters` 在一个分区的末端节点输入之后，为每个声明的状态追加一个恒定模式。核心把它们与观测一起计算，使用同一棵树、相同的分枝矩阵、类别权重和缩放因子。它们没有观测权重。以下是上一个示例的树上的四个可变性状：

```rust
use rove::ascertainment::VariableCharacters;
use rove::data::GapTreatment;
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling, nexus};

let input = "#NEXUS
BEGIN DATA;
  DIMENSIONS NTAX=3 NCHAR=4;
  FORMAT SYMBOLS=\"01\" MISSING=?;
  MATRIX
    A 0011
    B 0110
    C 1?01
  ;
END;";
let data = nexus::parse(input)?;
let sites = 0..data.character_count();
let patterns = data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)?.remove(0);
let variable = VariableCharacters::new(&patterns)?;

// 节点同上。先是四个观测模式，然后是两个恒定模式。
let mut core = LikelihoodCore::new(2, variable.pattern_count(), 1, 5, Scaling::Always);
for (tip, partials) in variable.tip_partials().iter().enumerate() {
    core.set_tip_partials(tip, partials);
}
let model = Mk::uniform(2);
let mut matrix = vec![0.0; 4];
for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    model.transition_matrix(length, &mut matrix);
    core.set_node_matrix(node, 0, &matrix);
}
core.calculate_partials(0, 1, 3);
core.calculate_partials(3, 2, 4);

core.integrate_partials(4, &[1.0]);
let pattern_log_likelihoods = core.calculate_log_likelihoods(model.frequencies());
let log_likelihood = variable.log_likelihood(pattern_log_likelihoods)?;
assert!((log_likelihood - -6.353478344891801).abs() < 1e-12);
```

`log_variable_probability` 单独返回校正项 `ln(1 − Σₛ Pₛ)`。

`VariableCharacters::new` 会拒绝可能为恒定的性状：所有单元格（包括缺失和不确定的单元格）都允许同一个状态。`0 ? 0` 和 `{01} 1 1` 会产生一个指出该性状的错误。`0 {12} 0` 会被接受，因为无论不确定单元格取哪个状态，它都是可变的。被接受的性状的每一种补全都是可变的，因此结果是在完整性状可变这一条件下观测的似然。BEAST 2.7.8 对与其恒定列完全相同的观测列另行处理：发出警告并将其权重设为零。

每个声明的状态都有自己的恒定模式，包括没有任何分类单元表现出的状态。各分区分别条件化。以简约信息性状为条件，以及针对缺失数据的逐性状校正，均未实现。

恒定模式与观测经过同样的剪枝。当它们几乎占据全部概率时，例如在极短的树上，它们的舍入误差会被放大 `Σₛ Pₛ / (1 − Σₛ Pₛ)` 倍；在两个存档的参考状态上，这个倍数都超过 100。对相距 `d` 的两个分类单元，每个条件化对数概率的误差大致按 `10⁻¹⁶ / d` 增长。如果恒定模式的总概率与一只差几个舍入误差，结果是 `ascertainment::Error::UndefinedCorrection`，而不是一个数值。

在[参考分析](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md)的两个存档 BEAST 2.7.8 状态上，Rove 重现了每个模式的对数似然、校正项和 Mkv 对数似然。最大的差异为 4 × 10⁻¹²，出现在 Mkv 值上。

## 重缩放

传入 `Scaling::Always`，每个内部节点的部分似然在计算时会除以其逐模式的最大值。每个最大值的对数被保留并在最后加回，因此结果与不缩放的计算一致，又没有深树上的下溢。`Scaling::None` 跳过这项工作。

## 检验

`cargo test` 运行 `tests/equality.rs` 中的属性测试。它们生成 2 到 10 个末端节点、2 到 6 个状态、1 到 4 个速率类别、最多 40 个模式的随机树，并在四种组合下把核心与 `reference::Problem` 比较：末端节点状态或末端节点部分似然，有或无重缩放。另有一个单独的测试检查 `restore` 能找回提议之前计算出的似然。

`tests/ascertainment.rs` 覆盖 Mkv 校正：存档的 BEAST 状态、在 2 到 4 个末端节点的树上对模式的完全枚举、两个分类单元在 10⁻⁸ 到 1000 距离上的闭式解、与 `reference::Problem::variable_log_likelihood` 比较的随机树、可能为恒定的性状、被拒绝的提议之后的恢复，以及枝长为零的树上未定义的校正。
