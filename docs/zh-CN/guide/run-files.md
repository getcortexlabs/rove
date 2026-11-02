# 运行文件夹指南

在结果或 History 中，点击文件夹旁的 **What are these files?**，即可离线查看说明。单链运行通常包含以下文件：

```text
analysis-<id>/
  setup.rove.json
  session.json
  events.jsonl
  engine.stderr.log
  results/
    run.json
    trace.log
    trees.nexus
    checkpoint.json
    checkpoint-<step>.bin
```

| 文件 | 用途 |
| --- | --- |
| `setup.rove.json` | 请求的配置、矩阵路径和指纹，可重新打开以准备新运行。 |
| `session.json` | 应用会话、状态和执行信息。 |
| `events.jsonl` | 进度事件，每行一个 JSON 对象。 |
| `engine.stderr.log` | 排查问题时使用的引擎消息，不是数值采样轨迹。 |
| `dating.json` | 启用化石定年时保存的定年输入。 |
| `results/run.json` | 引擎配置、构建与输入指纹，以及完成记录。 |
| `results/trace.log` | 采样数值，可在 Rove Diagnostics 或 Tracer 中打开。 |
| `results/trees.nexus` | 原始树样本，包括初始状态。 |
| `results/checkpoint.json` 和 `checkpoint-<step>.bin` | 恢复运行的索引与二进制状态。保留索引指向的二进制文件及关联输出。 |

多链运行将各链的文件保存在 `results/chain-01/`、`chain-02/` 等目录中。链组记录仍为 `results/run.json`。

## 从样本到树

可使用应用内公开的 20 × 50 示例矩阵。运行结束后先检查 **Diagnostics**，比较独立运行，然后选择 **Summarize trees**，确定舍弃多少初始样本（burn-in）。在 Rove 或 FigTree 中打开 `summaries/summary-<id>/mcc.tree`，查看带注释的代表树。`samples.trees` 保存原始样本；`summary.json` 记录摘要选项和支持度计算。生成摘要或完成请求步数都不能证明收敛。

`.nexus` 和 `.trees` 都可能是 NEXUS 树样本的扩展名。改名不会转换格式，也不会计算节点支持度。

## 后续报告

使用相应工具后，会生成可选的 `diagnostics/`、`summaries/`、`comparisons/`、`predictions/` 和 `marginal/` 目录。诊断包含 `diagnostics.json`。[后验预测检验](./posterior-prediction)包含 `predictive.json`、`replicates.jsonl`、`characters.tsv` 和 `matrices/` 中的模拟矩阵。报告应与原运行一起保存。导入的树样本使用 `tree-source.json` 记录原文件路径和指纹。

## 保留与共享

恢复运行需要完整文件夹、未改动的原矩阵和同一 Rove 构建。应用**不会**将矩阵或引擎复制到运行文件夹。配置或报告 JSON 无法替代二进制检查点。若分析使用筛选后的数据，需保留配置记录的确切矩阵。

共享树图时，可提供 `mcc.tree` 和摘要选项。检查采样时，应提供轨迹和原始树。复现分析需要完整背景与模型配置。共享前检查路径及敏感数据。部分完成或中断的运行可能包含不完整输出，应先检查状态再解释结果。
