---
description: "Rove 似然核心今天计算什么，以及如何从 Rust 调用它：末端节点、转移矩阵、部分似然、重缩放和对数似然。"
---

# 似然核心

`LikelihoodCore` 在给定每条分枝、每个速率类别各一个转移矩阵的前提下，为一个位点模式矩阵计算有根二叉树的对数似然。本页描述代码仓库中现有的 API。crates.io 上还没有 crate；要试用，请把代码仓库作为 Git 依赖使用。

::: info 范围
核心接收转移矩阵和末端节点数据，返回似然。独立的 [NEXUS 读取器](./nexus)负责准备观测和模式权重。树提议和 gamma 速率的计算仍是未来的工作。见[路线图](./roadmap)。
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

## 重缩放

传入 `Scaling::Always`，每个内部节点的部分似然在计算时会除以其逐模式的最大值。每个最大值的对数被保留并在最后加回，因此结果与不缩放的计算一致，又没有深树上的下溢。`Scaling::None` 跳过这项工作。

## 检验

`cargo test` 运行 `tests/equality.rs` 中的属性测试。它们生成 2 到 10 个末端节点、2 到 6 个状态、1 到 4 个速率类别、最多 40 个模式的随机树，并在四种组合下把核心与 `reference::Problem` 比较：末端节点状态或末端节点部分似然，有或无重缩放。另有一个单独的测试检查 `restore` 能找回提议之前计算出的似然。
