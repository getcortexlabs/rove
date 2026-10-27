---
description: "在 Rust 中读取形态学 NEXUS 矩阵，保留观测语义，并在显式的模型分区内压缩重复列。"
---

# 读取形态学 NEXUS

`nexus::parse` 把一个 UTF-8 字符串读入 `data::Matrix`。它保留分类单元的顺序、声明的状态字母表、性状标签和状态标签，以及每一个观测单元格。它没有运行时依赖，也不会调用 BEAST。

在代码仓库根目录下试试随附的参考矩阵：

```sh
cargo run --release --example read_nexus -- fixtures/reference/20x50/matrix.nex
```

```text
20 taxa, 50 characters, 2 declared states
23 observed patterns (50 total weight)
```

这个示例把所有性状当作一个分区，并在准备似然输入时拒绝缺口（gap）和多态。它只导入数据，不运行推断。

## 支持的语法

无效或不支持的输入会返回 `nexus::Error`，带有从 1 开始计数的行号和列号以及说明。

| 特性 | 支持情况 |
| --- | --- |
| 块 | 一个 `DATA` 或 `CHARACTERS` 块，前面可以有一个 `TAXA` 块。`TREES` 块会被跳过；树不会被导入。其他块会被拒绝。 |
| 维度 | 正的 `NTAX` 和 `NCHAR`。前置的 `TAXA` 块可以提供 `NTAX`。`NEWTAXA` 要求有 `NTAX`。所有声明的分类单元都必须在矩阵中恰好出现一次。 |
| 格式 | `DATATYPE=STANDARD`、`SYMBOLS`、`MISSING`、`GAP`、`RESPECTCASE`、`LABELS`、`NOTOKENS`，以及 `INTERLEAVE` 或 `INTERLEAVE=YES/NO`。 |
| 字母表 | 至少两个有效的 ASCII 符号，每个状态一个字符。默认为 `01`。声明的顺序会被保留，包括未观测到的状态。 |
| 顺序行 | 带名字的行，单元格之间可以有空白，可以换行。 |
| 交错行 | 每一页都以相同顺序重复每个分类单元的标签。每行占一行，同一页内各行的性状数相等。各页的宽度可以不同。空行可有可无。不支持没有标签的续页。 |
| 名字 | 单引号括起的名字、双写的撇号和不带引号的名字。下划线变成空格。匹配和重复检测忽略 ASCII 大小写；返回的名字保留原有写法。 |
| 标签 | 恰好有 `NCHAR` 个名字的 `CHARLABELS`；带索引的 `STATELABELS` 和 `CHARSTATELABELS`。描述性的状态标签不会缩小声明的字母表。 |
| 注释 | 嵌套的方括号注释，包括词内的注释。接受 LF、CRLF 和 CR 换行。 |

有 `TAXLABELS` 时，返回的行遵循声明的顺序，即使矩阵中的行以另一种顺序出现。否则，由矩阵的第一页决定顺序。

默认值是：状态不区分大小写，缺失符号为 `?`，没有缺口符号，顺序行。因此除非用 `GAP=-` 声明，`-` 不是缺口。在所选的大小写策略下，状态、缺失和缺口符号必须互不相同。

分子或连续数据类型、`TOKENS`、`TRANSPOSE`、`NOLABELS`、`EQUATE`、`MATCHCHAR`、状态范围、频率/计数、`ELIMINATE`、多个矩阵和不支持的命令都会被拒绝。`SETS`、`ASSUMPTIONS` 和应用程序特有的块同样会被拒绝：它们的分区、权重、排序或排除项不能在导入时消失。XML 解析不在这个 API 的范围内。

## 观测与末端节点似然

花括号与圆括号的区别遵循 [NEXUS 规范](https://doi.org/10.1093/sysbio/46.4.590)。对于声明的字母表 `012`：

| 输入 | 存储的单元格 | 末端节点似然向量 |
| --- | --- | --- |
| `1` | `Cell::State(1)` | `[0, 1, 0]` |
| `{01}` | `Cell::Uncertain([0, 1])` | `[1, 1, 0]` |
| `(01)` | `Cell::Polymorphic([0, 1])` | 在似然转换时被拒绝 |
| `?` | `Cell::Missing` | `[1, 1, 1]` |
| 声明的缺口 | `Cell::Gap` | 被拒绝，或在显式指定 `GapTreatment::Missing` 时为 `[1, 1, 1]` |

不确定表示所列状态中的一个；多态则记录同时存在多个状态。当前的 Mk 观测模型不表示多态。导入时会保留它以供检查，但压缩会返回一个错误，指出所涉及的分类单元和性状。不会自动转换为不确定。缺口同样需要显式选择，因为形态学中的缺口可能表示不适用。

不确定向量是似然掩码，不是概率分布：`{01}` 为每个允许的状态贡献 1，不除以二。状态集合在导入时会排序并去重。

## 压缩重复列

分区范围从 0 开始计数、连续、非空，并且必须按顺序恰好覆盖矩阵一次。只要性状使用不同的模型或参数，就传入单独的范围。每个分区都保留完整的声明字母表；读取器从不根据观测推断每个性状的状态数或有序模型。

```rust
use rove::{data::GapTreatment, nexus};

let input = std::fs::read_to_string("fixtures/reference/20x50/matrix.nex")?;
let data = nexus::parse(&input)?;
let sites = 0..data.character_count();
let partitions = data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)?;
let patterns = &partitions[0];

assert_eq!(patterns.weights().len(), 23);
assert_eq!(patterns.weights().iter().sum::<usize>(), 50);
```

`Patterns::weights` 保存精确的整数重数。`site_to_pattern` 把分区内的每个原始性状映射到一个模式，模式按首次出现的顺序编号。`tip_partials` 包含每个分类单元的位点模式主序值，可以直接交给 `LikelihoodCore::set_tip_partials`。调用核心的 `log_likelihood` 时，把整数权重转换为 `f64`。

压缩比较完整的单元格观测，包括其类别。缺失单元格和不确定单元格即使似然掩码相同，也保持为不同的模式。跨分区的相同列也保持分开。这样通过矩阵和映射保留了原始观测，代价是偶尔保留一些冗余计算。

只有观测到的列会被压缩。恒定列和仅含缺失的列仍然保留，也不会添加用于确认偏差校正的人造模式。[Mkv 校正](./kernel#以可变性状为条件)会单独追加它的恒定模式，并拒绝可能为恒定的性状。

测试读取两份 [20 × 50 fixture](./fixtures)，根据模式映射重建它们的列，并在带有不确定和缺失数据的生成矩阵上比较压缩前后的似然。
