---
description: "随 Rove 提供的合成对照矩阵、重新生成并降级它的脚本，以及如何在 BEAST 2 中运行它。"
---

# 合成矩阵

`fixtures/synthetic/control_20x50.nex` 是一个阳性对照：20 个分类单元，50 个二态性状，没有同塑性，没有缺失数据，树已知。T20 是外群，全为零。内群树是平衡的，它的 18 个支系每一个都有专属的共有衍征，九个最大的支系各三个，其余各两个。最后五个性状是自有衍征。这种构造并不保证后验支持：置根、时钟模型和先验同样影响结果。

`control_20x50.tre` 是 Newick 格式的真实树，也作为注释重复在 NEXUS 文件末尾。`control_20x50.legend.txt` 把每个性状映射到它所支持的支系。

## 重新生成与降级

`tools/synth_matrix.py` 重新生成对照并产出降级变体：

```sh
python3 tools/synth_matrix.py --out fixtures/synthetic/control_20x50
python3 tools/synth_matrix.py --homoplasy 10 --seed 7 --out /tmp/homoplasy10
python3 tools/synth_matrix.py --missing 0.2 --out /tmp/missing20
python3 tools/synth_matrix.py --drop T1-T4 --out /tmp/no_t1_t4
```

降级项列在图例文件的末尾。

## 在 BEAST 2 中运行

在 BEAUti 中安装 MorphModels 包，导入 NEXUS 文件，选择带可变性状确认偏差校正的 Lewis Mk 模型、严格时钟和 Yule 树先验。

这是针对生成对照的一个示例设置。它与下面选定的参考分析不同。

## 随附的参考矩阵

`fixtures/reference/20x50/matrix.nex` 是一个独立的数据集，带有其原始图例和 BEAST XML。它的拓扑和性状分配与生成的对照不同。记录在案的 BEAST 2.7.8 实验使用松弛时钟、四个 gamma 类别、生灭先验和内群单系约束。输入、来源和结果见[参考报告](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md)。

[NEXUS 读取器](./nexus)独立导入这两份 fixture。每份都有代表 50 个性状的 23 个观测模式，但模式各不相同。在验证中，两个矩阵都不会用来替代对方。
