---
description: "根据形态性状的采集方式选择 Mk、Mkv 或 Mkparsinf。"
---

# 性状选择

校正必须与哪些性状可以进入矩阵相符。它不是速度或质量选项。

| 采集时保留的性状 | CLI 选项 | Mac 应用 |
| --- | --- | --- |
| 恒定及可变性状 | `--conditioning none` | All characters · Mk |
| 仅可变性状 | `--conditioning variable`（默认） | Variable only · Mkv |
| 仅简约信息性状 | `--conditioning informative` | Informative only · Mkparsinf |

完整性状中至少有两个不同状态，且每个状态至少出现于两个分类单元时，该性状具有简约信息。`0011` 符合，`0001` 和 `00123` 不符合。对于多状态性状，仅排除恒定模式和只有一个末端不同的模式并不足够。

Mkparsinf 支持两种时钟预设、按状态数分区、独立链和检查点。每个分区使用等频率无序 Mk 模型。Rove 在相同树和分支速率下先对速率类别求混合，再以信息性状出现的概率校正观测似然。算法缓存子树概率，无需枚举被排除的列。该递推不适用于有序或不等频率模型。

只有在缺失及不确定单元的所有允许补全均具有简约信息时，才接受该性状：`0011?` 符合，`001??` 不符合。条件事件针对所有分类单元的完整性状，而非每种缺失数据掩码各自的事件。`rove inspect` 提供 `informative_error`；应用阻止不兼容的分析，CLI 指明原始性状编号。Rove 不会自动删除性状。若树极短，保留概率在双精度下无法与零区分，则拒绝该树。

在应用中打开 **Model → Which characters did you include?**，仅当采集方式相符时选择 **Informative only · Mkparsinf**。内置的 20 × 50 示例含有不具简约信息的性状，请继续为它使用 Mkv。

简短的功能检查：

```sh
target/release/rove sample fixtures/validation/informative/matrix.nex \
  --conditioning informative --model reference \
  --output runs/informative --seed 42 --steps 10000 --sample-every 100
```

此短运行检查工作流程，不证明收敛。[验证](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/informative)包括完整模式枚举、缺失数据检查、检查点恢复以及与 BEAST 2.7.8 在固定状态下的一致性。它不证明后验分布一致，也不证明推断加速。
