---
description: "在明确数据来源、分类单元匹配和相对速率的前提下联合分析形态与已比对 DNA。"
---

# 联合分析形态与 DNA

在 Mac 欢迎页选择 **Combine morphology and DNA**，指定一个 STANDARD 形态
NEXUS 文件及一个已比对的 DNA NEXUS/FASTA 文件，检查缺失分类单元的处理方式，
保存新的 `.mixed.json` 输入。CLI 对应操作：

```sh
rove combine --morphology morphology.nex --dna alignment.fasta --missing-taxa reject --output study.mixed.json
rove sample study.mixed.json --output combined-run --seed 42 --steps 30000000 --sample-every 3000 --model reference --morphology-rate estimate --morphology-rate-log-sd 1
```

清单内嵌两个原始 UTF-8 文本及其 SHA-256 指纹，不读取外部源路径。原文件移位后
仍可使用。不会覆盖现有输出；每次导入都检查格式、版本、数据类型和指纹。
清单包含研究数据，只有获准分享两个原始数据集时才应分发。

分类单元按**标准化后的完整名称精确匹配**，不按行位置匹配。现有的下划线转空格
规则仍然适用。默认 `reject` 要求两组分类单元相同；显式选择
`--missing-taxa missing` 保留并集，为某来源没有的分类单元填入缺失数据，应用会
逐来源列出它们。至少需要两个共同分类单元。仅大小写不同的名称会被拒绝；
其他拼写差异也应先核对，不能直接视为生物学缺失。缺失单元不提供状态信息；
gap 保持独立的 `--gaps reject|missing` 策略。Rove 不自动进行序列比对。

每个数据块保留自己的字母表、字符顺序、标签和观测。形态支持观察、声明或完整
状态标签定义的字母表、有序/无序模型和 `--conditioning none|variable|informative`。
这些选项**仅作用于形态**。DNA 使用无条件 JC69、完整四种碱基及全部位点，包括
常量位点。IUPAC 歧义表示可能碱基的掩码；真正的多态性仍会阻止推断。

两个数据块共享拓扑、分支时间、严格或对数正态分支时钟，以及启用时的 gamma
形状参数。速率类别在各位点分别积分。不提供按块独立的时钟或 gamma 形状。
DNA 相对速率固定为 1；所有形态分区共用正乘数 `r`，距离为
`时间 × 分支速率 × r`，DNA 则为 `时间 × 分支速率`。这确定相对尺度，并不把
未定年的分支长度变成日历年龄。化石定年继续使用明确的全局时钟及校准设置。

必须明确选择速率模型：

- `--morphology-rate 2.5` 将比率固定在该正值。
- `--morphology-rate estimate --morphology-rate-log-sd s` 使用
  `log(r) ~ Normal(0, s²)` 估计比率。省略时，对数标准差为 1；中位数为 1，
  均值为 `exp(s²/2)`。应评估结果对这一假设的敏感性。

标准化对数先验为
`−log(r) − log(s) − log(2π)/2 − (log(r)/s)²/2`。
尺度提议使用 `f ~ Uniform(a, 1/a)`，对数 Hastings 比为 `−log(f)`。
估计时 `morphology_rate_scale` 默认权重为 3；比率固定时不可使用该提议。
状态保存在 `trace.log` 和检查点中。联合对数似然为两块之和，其中形态使用选定
的抽样校正。先验只计一次，相对速率先验经过标准化，可用于边际似然估计。

格式 12 的应用配置保留来源、分类单元策略和速率选项。复核、历史、CLI 导入、
比较、恢复和边际估计均核验该身份。预测检验重建抽样时的比率，输出带类型的
`.mixed.json` 重复数据，保留各块字母表和缺失掩码。模拟歧义或多态性观测仍需
明确的观测模型。导出是否包含原矩阵必须显式选择；此选择也适用于预测检验
保存的原始副本。导出的模拟数据是合成数据，不能替代原始观测。

验证包括 BEAST.base 2.7.8 + MM 1.2.1 的 36 个固定状态、独立分区求和、先验
积分和稳态抽样、约束、缓存重算、精确检查点恢复、预测及原生应用执行。
参考似然容差为 1e-7，包含独立 gamma 分位数算法的近似差异。数据和生成器位于
`fixtures/validation/mixed` 与 `tools/beast/MixedStates.java`。这些是正确性检查，
不是收敛认证或速度比较。
