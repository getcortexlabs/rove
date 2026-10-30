---
description: "Rove 似然核心今天计算什么，以及如何从 Rust 调用它：末端节点、Mk 分枝距离、转移矩阵、部分似然、Mkv 校正、重缩放和对数似然。"
---

# 似然核心

`LikelihoodCore` 为一个位点模式矩阵计算有根二叉树的对数似然。每条分枝按速率类别给出，可以是 Mk 模型下的距离，也可以是转移矩阵。本页描述代码仓库中现有的 API。crates.io 上还没有 crate；要试用，请把代码仓库作为 Git 依赖使用。

::: info 范围
核心接收分枝距离或转移矩阵，以及末端节点数据，并返回似然。独立的 [NEXUS 读取器](./nexus)负责准备观测和模式权重，[Mkv 校正](#以可变性状为条件)则把似然条件化于可变性状。[参考模型](./reference-model)可生成四个 gamma 速率类别；树提议属于[采样器](./sampler)。见[路线图](./roadmap)。
:::

## 节点与缓冲区

节点从 `0` 编号到 `node_count − 1`。核心不知道哪些节点是末端节点；它只要求在每个内部节点的两个子节点就绪之后，为该节点调用一次 `calculate_partials`。

每个节点拥有两个部分似然缓冲区和两个分枝缓冲区。在一次提议中覆盖某个节点之前，先调用 `set_node_partials_for_update` 或 `set_node_matrix_for_update`，并用 `store`、`restore` 和 `unstore` 管理哪一组是当前的。无论分枝缓冲区存放的是距离还是矩阵，`set_node_matrix_for_update` 在两种核心中都会切换它。[设计](./design#双缓冲与重缩放)页解释这一机制。

## 末端节点

`set_tip_states(node, states)` 接收每个模式一个状态索引。等于或大于状态数的值表示缺失或不确定的数据：每个状态的部分似然都取 1.0。

`set_tip_partials(node, partials)` 接收 `patterns × states` 个值，按位点模式主序排列，用于带不确定性的末端节点。

两者都写入该节点的两个缓冲区，因此末端节点在 `restore` 之后依然保留。

## Mk 分枝

`LikelihoodCore::mk(&model, patterns, categories, nodes, scaling)` 为一个 `Mk` 模型构建核心。`set_node_distance(node, category, distance)` 把 `node` 上方的分枝在一个速率类别中设为某个距离，单位是每个性状的期望变化次数。速率异质性由调用方处理：传入枝长乘以该类别的速率。

核心从不构建转移矩阵。Mk 的矩阵是 `P = e·I + (1 − e)·1πᵀ`，其中 `e = exp(−d/β)`，而 `β = 1 − Σᵢ πᵢ²` 是使每单位距离期望发生一次变化的归一化常数。核心为每条分枝存放 `e` 和 `1 − e`，并用下式更新子节点的部分似然 `L`：

```text
(P·L)ᵢ = e·Lᵢ + (1 − e)·Σⱼ πⱼ·Lⱼ
```

加权和对每个模式只计算一次，由所有状态共用，因此更新一个子节点在每个模式、每个速率类别上的代价是 O(k)，而不是矩阵乘积的 O(k²)。`1 − e` 由 `f64::exp_m1` 计算，在短分枝上保持精度。

`Mk::uniform(k)` 使用相等的频率。`Mk::with_frequencies` 接收不相等的频率：同样的形式适用于任何 equal-input 模型，即一次变化按 `π` 抽取新状态。除非至少有两个频率，且每个都为正、有限，并在舍入误差范围内和为一，否则它返回 `model::Error`。频率按原样使用，不做归一化，因此舍入到较少位数的值会被拒绝，例如三个 0.3333333：请先除以它们的和。与矩阵核心一样，`log_likelihood` 仍然接收根节点频率；请传入 `model.frequencies()`。

对 Mk 核心调用 `set_node_matrix` 或 `node_matrix` 会 panic；对矩阵核心调用 `set_node_distance` 或 `node_distance` 也会 panic。传入负数或 NaN 距离时，`set_node_distance` 和 `transition_matrix` 也会 panic。

## 转移矩阵

`LikelihoodCore::new(states, patterns, categories, nodes, scaling)` 则构建接收矩阵的核心。`set_node_matrix(node, category, matrix)` 为 `node` 上方的分枝、在一个速率类别中，存放一个 `states × states` 的行主序矩阵，核心对它做完整的矩阵乘法。Mk 以外的模型走这条路径。`model::Mk` 也能构建自己的矩阵：

```rust
use rove::model::Mk;

let model = Mk::uniform(2);
let mut matrix = vec![0.0; 4];
model.transition_matrix(0.1, &mut matrix);
```

`transition_matrix(distance, out)` 为一条分枝填充 `out`，分枝长度以每个性状的期望变化次数给出，所用的 `e` 和 `1 − e` 与 Mk 核心相同。

## 计算似然

一个二态性状的四个模式，分布在三个末端节点上，树为 `((A, B), C)`：

```rust
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling};

// 节点：A = 0，B = 1，C = 2，(A, B) = 3，根 = 4。
let patterns = 4;
let categories = 1;
let nodes = 5;

let model = Mk::uniform(2);
let mut core = LikelihoodCore::mk(&model, patterns, categories, nodes, Scaling::Always);
core.set_tip_states(0, &[0, 0, 1, 1]);
core.set_tip_states(1, &[0, 1, 1, 0]);
core.set_tip_states(2, &[0, 1, 0, 2]); // 2 超出范围：缺失数据

for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    core.set_node_distance(node, 0, length);
}

core.calculate_partials(0, 1, 3);
core.calculate_partials(3, 2, 4);

let pattern_weights = [1.0, 1.0, 1.0, 1.0];
let log_likelihood = core.log_likelihood(4, &[1.0], model.frequencies(), &pattern_weights);
assert!((log_likelihood - -8.957188856221643).abs() < 1e-12);
```

矩阵核心 `LikelihoodCore::new(2, patterns, categories, nodes, Scaling::Always)` 使用 `model.transition_matrix` 给出的矩阵时，返回的值在舍入误差内相同。

`log_likelihood` 按类别权重对根节点的部分似然做积分，乘以状态频率，逐模式取对数，加上所有缩放因子，再按模式权重求和。这些步骤也可以分别通过 `integrate_partials`、`calculate_log_likelihoods` 和 `pattern_log_likelihoods` 单独调用。

参考实现对这棵树给出相同的值。这就是等价性测试在随机输入上所检查的。

## 以可变性状为条件

形态学矩阵通常只保留在分类单元之间有变化的性状。它们的似然必须以这种选择为条件，如 Lewis 的 [Mkv 模型](https://doi.org/10.1080/106351501753462876)。设 `Pₛ` 为所有分类单元都处于状态 `s` 的概率，并已按速率类别混合；似然为 `Lᵢ`、权重为 `wᵢ` 的每个观测模式 `i` 贡献

```text
wᵢ · (ln Lᵢ − ln(1 − Σₛ Pₛ))
```

`ascertainment::VariableCharacters` 在一个分区的末端节点输入之后，为每个声明的状态追加一个恒定模式。核心把它们与观测一起计算，使用同一棵树、相同的分枝、类别权重和缩放因子。它们没有观测权重。以下是上一个示例的树上的四个可变性状：

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
let model = Mk::uniform(2);
let mut core = LikelihoodCore::mk(&model, variable.pattern_count(), 1, 5, Scaling::Always);
for (tip, partials) in variable.tip_partials().iter().enumerate() {
    core.set_tip_partials(tip, partials);
}
for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    core.set_node_distance(node, 0, length);
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

在[参考分析](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md)的两个存档 BEAST 2.7.8 状态上，Rove 重现了每个模式的对数似然，差异不超过 1.4 × 10⁻¹²；Mkv 对数似然的差异不超过 2.2 × 10⁻¹²。逐模式的差异几乎全部来自 BEAST 自身的舍入：它的 LewisMK 模型把 `1 − e` 算作 `1 − exp(−x)`，在参考数据的短分枝上会丢失有效数字。与用 60 位有效数字对相同输入求得的精确值相比，Rove 的逐模式对数似然相差不超过 6 × 10⁻¹⁵，BEAST 的则不超过 1.4 × 10⁻¹²。

## 重缩放

传入 `Scaling::Always`，在每次内部节点更新后重缩放。对于每个模式在所有状态和类别中的有限正最大值 `m`，从 IEEE 754 指数位得到 `e = clamp(floor(log2(m)), -1023, 1022)`，并将所有部分似然乘以 `2^-e`。这里使用较低的二次幂，不舍入到最近的二次幂。该范围保证乘数有限且为正规数，也适用于次正规最大值。最大值为零或非有限值时使用 `e = 0`；不可能的模式仍返回负无穷。

每个缓冲区以 `i64` 保存子树的累计指数：`E = e + E_left + E_right`。在根节点按类别和状态积分后，模式的对数似然为 `ln(L_scaled) + E_root * ln(2)`。这避免了逐节点对数计算和反复对所有节点求和。整数总和不会积累浮点误差，store/restore 同时选择对应的部分似然和指数。`Scaling::None` 跳过重缩放。

舍入与此前按最大值缩放的方法不同，但统计模型不变。可移植后端与 NEON 逐位一致，NaN 载荷除外。检查点保存新的指数格式，仍要求使用创建检查点的同一个可执行文件来恢复运行。

测试覆盖所有二进制指数边界及 36 个独立的 60 位精度用例，包括 2、3、4、8 个状态，一或四个类别，缺失数据，`1e-9` 到 `100` 的距离，以及不缩放时似然会下溢的 2,048 末端平衡树和梯状树。详见[测量报告](https://github.com/getcortexlabs/rove/blob/main/benchmarks/scaling/2026-10-30/README.md)。

## 后端

核心运行两种后端之一。两者给出相同的结果：所有非 NaN 的值逐位相同，而 NaN 的载荷或符号可能不同。`Backend::Portable` 是适用于任何目标平台的普通 Rust；在 release 构建中，编译器会逐个循环地对它进行向量化。`Backend::Neon` 用于 aarch64，它以八个模式为一块、一次遍历完成 Mk 的节点更新：加权和保存在寄存器中；核心重缩放时，每个模式的当前最大值在每个速率类别之内也保存在寄存器中。对每个模式，它执行与可移植代码相同的浮点运算，顺序也相同，不使用融合乘加。矩阵乘法和根节点计算在两种后端下使用相同的代码。NEON 还通过向量整数运算提取二进制缩放指数。

新建的核心使用 `Backend::NATIVE`：目标平台启用 NEON 时用 NEON，否则用可移植代码。`backend` 返回当前后端，`set_backend` 更改它；请求当前构建不包含的后端会 panic，`Backend::is_available` 可以查询构建包含哪些后端。

## 检验

`cargo test` 运行 `tests/equality.rs` 中的属性测试。它们生成 2 到 10 个末端节点、2 到 6 个状态、1 到 4 个速率类别、最多 40 个模式的随机树，并把两种核心都与 `reference::Problem` 比较：末端节点状态或末端节点部分似然，有或无重缩放。其中一个测试让枝长分布在 10⁻⁹ 到 100 之间。另一个施加随机的分枝变化序列，逐一接受或拒绝，并在每一步之后检查似然；被拒绝的变化必须恢复出与之前完全相同的值。一个只含两个分类单元的测试检查两者相距短至 10⁻¹⁵ 时发生一次变化的概率，`src/model.rs` 中的单元测试检查频率验证和速率归一化。另一个属性测试在构建包含的每种后端上运行相同的更新、保留的变化和被拒绝的变化，并要求每个部分似然、每个模式的对数似然和每个总对数似然都逐位相同。还有一个测试把带符号的零、次正规数、无穷大和 NaN 输入两种后端，只允许两个 NaN 之间存在差异。

`tests/ascertainment.rs` 覆盖 Mkv 校正：存档的 BEAST 状态（同时对照 BEAST 的值和 60 位有效数字的求值）、在 2 到 4 个末端节点的树上对模式的完全枚举、两个分类单元在 10⁻⁸ 到 1000 距离上的闭式解、与 `reference::Problem::variable_log_likelihood` 比较的随机树、可能为恒定的性状、被拒绝的提议之后的恢复，以及枝长为零的树上未定义的校正。

## 实验性子树重复模式

Cargo 功能 `experimental-repeats` 为至少有 64 个模式的 Mk 核心启用研究原型。它将子树内观测相同的模式分组，使用现有 Mk 核心计算代表模式，再把结果展开到普通缓冲区。每个核心维护独立索引，因此分区、状态字母表、模型和速率类别保持分离。只有能省去至少一半模式位置（含填充）时，节点才使用打包计算。

末端向量按精确值比较；内部类别由两个子节点的类别组成。分支和速率变化可复用类别。拓扑变化重建受影响的索引；拒绝提案时，索引选择器与似然缓冲区一起恢复。从检查点继续运行时，根据保存的树和原始观测重建索引。

[评估报告](https://github.com/getcortexlabs/rove/blob/main/benchmarks/repeats/2026-10-30/README.md)记录了初始化、拓扑维护、内存、分配和完整采样器运行。部分可移植计算的加速未能稳定延续到 NEON 或完整采样。默认构建和原生应用均关闭此功能。`disable_site_repeats` 将核心切换为稠密更新；`repeat_stats` 提供原型计数器。该实现保留完整的部分似然缓冲区，并额外使用索引与打包内存；它未实现按节点紧凑存储。
