---
description: "使用 JC69 分析 NEXUS 和 FASTA 中已比对的 DNA，明确处理模糊碱基与缺口。"
---

# 使用 JC69 分析已比对 DNA

在应用中打开 NEXUS `DATATYPE=DNA` 矩阵或已比对的 FASTA 文件。导入自动选择
**DNA · JC69**，每个位点保留 A/C/G/T 四个可能状态，并保留恒定位点。
Rove 不执行序列比对、裁剪或测序读段组装。相同输入可用于 `sample`、`inspect`
及 `marginal`：

```sh
rove sample alignment.fasta --output dna-run --seed 42 --steps 30000000 --sample-every 3000 --model reference --gaps missing
```

仅在缺口不应提供状态信息时选择 `--gaps missing`；默认拒绝缺口。`?` 表示缺失，
`R` 允许 A/G，`Y` 允许 C/T，其他 IUPAC 代码保留其可能状态，`N` 允许全部四种。
这些是取值为一的似然掩码，不是分数概率或额外状态。括号内的多态被保留，但
不能用于推断。DNA 默认使用 `--conditioning none --partitioning declared`；
显式请求 Mkv、Mkparsinf 或形态学状态重编码会被拒绝。

FASTA 序列可跨行，必须非空且等长。按照 [FASTA 标题约定](https://www.ncbi.nlm.nih.gov/genbank/fastaformat/)，
`>` 后到首个空白之前的标识符是分类单元名称，其余描述被忽略。Rove 现有的
NEXUS 命名规则也适用于 FASTA：下划线转换为空格，重复检查忽略 ASCII 大小写，
其余拼写不变。定根或匹配数据前请核对界面显示的名称；原始文件不会修改。
NEXUS DNA 碱基不区分大小写，固定为 A/C/G/T，默认缺口为 `-`，可选
`SYMBOLS="ACGT"`。自定义字母表、RESPECTCASE、有序 DNA、RNA、蛋白质及不支持
的指令均被拒绝。`MISSING=N` 可显式将 N 定义为缺失。支持与形态学相同的顺序
或带标签交错布局。

JC69 固定各碱基频率为 1/4，交换率相等，并归一化为每个位点一次预期替换：
`Pii(t)=1/4+3/4 exp(−4t/3)`，不同碱基为 `Pij(t)=1/4−1/4 exp(−4t/3)`。
现有控件提供严格或对数正态松弛时钟、伽马类别、定根及支持的树先验。DNA 不使用
形态学取样校正。本次交付不含 HKY、GTR 或混合数据推断。

格式 11 的设置、历史、运行记录和检查点保存 DNA/JC69 及缺口策略，旧版应用会
拒绝这些设置。命令行运行可导入历史；不兼容模型不能合并做收敛比较。树、诊断和
边际似然估计保留拟合模型。短执行测试验证集成，不证明后验收敛。后验预测目前
要求单个碱基或缺失单元；IUPAC 不确定观测需要明确的观测模型，模拟时会拒绝。
模拟副本使用 DNA NEXUS。复现导出将原始输入字节保存在 `matrix.fasta` 或 `matrix.nex` 中。
矩阵须显式选择才会导出，此选项同样控制预测检查保存的原始矩阵副本。

`fixtures/validation/dna` 包含合成数据、独立解析内部状态求和、BEAST.base 2.7.8
的六个固定状态、模拟频率与精确续跑测试。使用相同类别率时，对数似然绝对误差
小于 1e-10；分别计算伽马分位数时采用 1e-7 容差，BEAST 的 AS91 近似在这些状态
的类别率差异小于 3e-8。这是数值验证，不是性能比较。
