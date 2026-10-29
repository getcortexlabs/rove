---
description: "构建并体验 Rove 的首个原生 Mac 工作区：导入矩阵、检查性状并准备分析。"
---

# Mac 预览版

Rove 已有使用 SwiftUI 和 AppKit 构建的首个原生 macOS 工作区。
它可以导入矩阵、准备设置，并运行内置 Rust 引擎，显示实际进度且支持取消。
无需 BEAST 或 Java。

![Rove 原生工作区](/images/macos-preview.png)

## 构建与打开

在 macOS 14 或更高版本上安装支持 Swift 6 的 Xcode 和 Rust，
然后从代码仓库根目录运行：

```sh
scripts/build-macos.sh
open build/macos/Rove.app
```

这会构建带有 ad hoc 签名的本地应用，不是经过公证的发行版或安装程序。
界面的深色外观参考 Cortex Labs，支持英语和巴西葡萄牙语。

## 语言与设置

从侧边栏打开 **Settings**，或按 **Command-逗号**。可以跟随 Mac 的语言，
也可以选择英语或巴西葡萄牙语。切换立即生效，并保留已打开的矩阵和分析设置。
偏好保存在本机上。还可以减少动画和侧边栏透明度；系统的辅助功能偏好始终
优先。系统对话框使用 Mac 的语言，引擎的原始诊断信息仍可在 **Technical details** 中查看。

**Rove → About Rove** 显示项目起源、应用版本和贡献者：Rafael Augusto（软件与设计）
以及 Thiago J. Silva（生物学与研究）。窗口使用应用当前语言，并提供网站、指南和源码仓库链接。

## 探索矩阵

按 **Command-O** 打开 NEXUS 文件，将单个文件拖到欢迎界面，或选择
**Try an example**。示例是提供的参考矩阵，含 20 个末端和 50 个性状。

矩阵视图显示分类单元、观测性状、压缩模式和缺失单元格。点击单元格可查看
分类单元、性状和观测值。统计覆盖整个矩阵；可滚动预览最多显示 200 个分类
单元和 120 个性状。应用接受最大 32 MB 的文件。

能够导入不等于能够采样。多态矩阵可以查看，但当前采样器仅接受二态性状。
不支持的输入会显示原因。把 gap 当作缺失数据需要明确选择；可能为恒定状态
的性状不能使用 Mkv。

## 准备并运行分析

选择严格时钟或[松弛时钟参考预设](./reference-model)、Mk 或 Mkv、链长、
采样间隔和随机种子。先验保持引擎默认值。在 Model 中，**Add group** 可选择
必须形成单系群的末端。各组必须相互包含或互不相交。应用不会自动施加任何
约束，包括示例的内群。约束所强制的支持不能视为数据独立恢复的证据。

在 **Outgroup** 中可以选择一个末端，例如 T20。所有末端都保留在分析中：
选择 T20 会要求 T1–T19 构成单系群，T20 位于根分叉的另一侧。
外群末端不是根处的祖先节点。此控件只支持一个外群，默认 **Not selected**。

在 **Additional groups** 中添加分组，或点击铅笔编辑已有分组。勾选只是准备
选择；**Apply group** 才会应用，**Cancel** 会放弃未应用的修改。
未勾选的末端仍参与分析。与外群冲突的分组会阻止运行。清除外群不会删除手动
分组，因此请在核对页面检查 **Applied groups**。

核对页面列出根分叉、实际应用的分组和全部保留末端。运行中的
**Applied analysis setup** 和 History 详情也显示这些信息。保存后重开及
**Use this setup** 会保留选择；导入其他矩阵或 **New analysis** 则从无外群
开始。旧配置保留原有分组，不会自动推断外群。新格式 2 配置需要更新后的应用。
修改配置只用于新分析，不会重定根已有结果。强制分组的支持来自约束本身。

![Mac 应用中的明确外群选择](/images/macos-outgroup.png)

核对设置和分组，选择输出位置，然后点击 **Start analysis** 或按
**Command-Return**。内置 Rust 引擎在本机运行，窗口显示实际完成的步数和已用
时间。运行期间仍可修改界面偏好，但科学配置会锁定。界面不会编造预计完成
时间、ESS 或后验支持。

每次运行创建新文件夹，默认位于 `~/Rove Runs`。**Open folder** 打开保存的配置、
会话记录、进度事件和引擎日志。`results` 子目录包含 `trace.log`、`trees.nexus`
和 `run.json`。运行结束后，Trace、Trees 和 Run record 按钮可在 Finder 中显示
相应文件。**Completed** 表示请求的步数已完成且输出元数据已核验，并不证明
收敛。解释结果前应比较独立运行并评估采样与树的诊断；参见[采样指南](./sampler)。

**Cancel analysis** 会在停止前请求确认。部分输出保留并明确标记，不能视为完整
分析或检查点。分析期间关闭主窗口或退出应用也会请求确认，并等待引擎退出。
强制停止可能留下不完整的文件。目前尚不能恢复中断的链。

## 保存并重新打开配置

**Save setup** 将原始矩阵路径、SHA-256、设置和明确选择的分组写入 `.rove.json`，
不会复制矩阵或启动运行。**Open Setup**（**Command-Shift-O**）会检查原始矩阵
是否存在且指纹一致，然后恢复配置。缺失或变更的输入会在执行前报告。
开始新运行前请核对恢复的设置。配置文件不是检查点。

**Copy terminal command** 生成使用内置 Rust 引擎的命令，并指定 `~/Rove Runs`
下的新文件夹。命令在执行时读取矩阵。相同的二进制文件、输入、设置、分组和
种子通过应用与 CLI 产生相同的科学样本；应用报告进度不会改变采样。

自动化可使用 `rove sample ... --events-json` 输出带版本号的 JSON 行，事件包括
`started`、`progress`、`completed`、`cancelled` 或 `failed`。
`--expect-input-sha256 HASH` 在创建输出前拒绝已变更的输入；
`--cancel-on-stdin-close` 在所有者关闭输入管道时请求取消。应用使用这些选项。
[协议与进程说明](https://github.com/getcortexlabs/rove/blob/main/apps/macos/README.md#cli-event-protocol-version-1)
定义字段、验证规则和退出码。

## 浏览历史运行

![Rove 历史记录中的已完成、已取消和失败的运行](/images/macos-history.png)

从侧边栏打开 **History**，或按 **Command-Shift-H**。运行按时间从新到旧排列，
显示矩阵、日期、记录的状态、已用时间、模型和种子。可按矩阵名或种子搜索，
也可按状态筛选。选择一条运行可查看已完成与请求的步数、设置、明确选择的
分组及原始输入指纹。文件按钮可在 Finder 中显示现存的输出。

应用会自动发现 `~/Rove Runs` 中以前的应用运行，并在本机记住通过应用选择的
其他输出位置。**Add run folder** 可添加保存在别处的应用运行；请选择包含
`session.json` 或 `setup.rove.json` 的文件夹。**Refresh** 重新检查文件。
应用只读取小型元数据文件，不读取轨迹或树的内容，并在界面线程之外完成读取。

缺失的文件夹、无法读取的记录和不完整的输出会明确标注。没有确认结束的运行
标记为 **Interrupted**，即使已有输出文件。旧记录仍可读取；没有记录的测量值
显示为横线，不作估算。新的尝试在检查输入前保存会话，因此立即取消或启动失败
也能留在历史中；指纹不符时不会产生科学分析输出。

**Use this setup** 会检查原始矩阵并打开保存的设置供核对，不会自动启动分析或
恢复中断的链。运行期间可以浏览 History，再通过 **Current analysis** 返回；
在当前分析结束或取消前，不能复用配置。**Back to workspace**
（**Command-Shift-R**）保留已打开的矩阵和设置。历史功能不会删除输出文件，
也不能证明收敛。

## 从命令行检查

```sh
target/release/rove inspect fixtures/reference/20x50/matrix.nex
```

命令输出带版本号的 JSON，包含计数、有限的单元格预览、输入哈希，以及分别
说明采样器和 Mkv 适用性的错误。只有该解释合适时才加上 `--gaps missing`。
读取矩阵不会写入分析结果。

## 汇总后验树

![Rove 的后验树汇总](/images/macos-summary.png)

分析完成后，在结果或 History 中选择 **Summarize trees**，设置 burn-in 百分比和节点高度，
再选择 **Create tree summary**。重新打开面板可以查看已保存的汇总。
计算在界面线程之外运行，可以取消。应用不汇总未完成、已取消或中断的分析。

Burn-in 丢弃最前面的 `floor(样本总数 × 百分比 / 100)` 棵树，向下取整。
`STATE_0` 也计为一个样本。默认的 10% 是可修改的建议，并非收敛诊断；
请检查轨迹后再选择。面板显示丢弃和保留的数量。

MCC（最大分支可信度）选择一个**已采样的拓扑**，不是多数规则共识树。
Rove 对每棵保留树的内部支系频率取对数并求和；求和前先对支系排序。
分数完全相同时，选择保留样本中最先出现的候选树。
`posterior` 是包含该支系的保留树所占比例。
由已记录约束强制形成的支系标有 `constrained=true`，其支持度并非从数据中独立恢复的结果。

**Keep selected tree** 保留选中样本的节点高度。
**Common ancestor mean** 使用每个组在**全部**保留树中的最近共同祖先高度的平均值。
枝长是父、子节点高度之差，单位沿用输入树的时间单位。速率仍是独立的乘数；
不推断日历年代，也不转换为速率缩放后的距离。
高度和速率的均值及范围以该支系出现为条件；`CAheight_mean` 和 `CAheight_range` 使用全部保留树。
范围表示观测最小值和最大值，**不是 HPD 或可信区间**。尚未实现高度中位数和 HPD 区间。

每次汇总在分析目录的 `summaries` 下创建新文件夹：

- `mcc.tree`：供 FigTree 使用的单棵有根、带注释的 NEXUS 树。
  在 FigTree 的 Node Labels 中选择 `posterior` 可显示支持度。
- `samples.trees`：全部原始样本的逐字节副本，包含 burn-in，供要求 `.trees` 扩展名的工具使用。
- `summary.json`：输入和可执行文件的哈希、原分析元数据、burn-in、保留数量、选中状态、方法、
  高度策略和 Rove 版本。

按钮会在 Finder 中定位文件。原始 `results/trees.nexus` 和其他分析输出保持不变。
每次汇总都需要容纳另一份原始树文件的磁盘空间。汇总和高支持度不能证明收敛；
还应比较独立运行及采样诊断。

独立 Rust 命令执行同样的计算，不依赖 Java 或 BEAST：

```sh
target/release/rove summarize /path/to/results/trees.nexus \
  --output /path/to/new-summary --burnin-percent 25 --heights ca
```

可改用 `--burnin-samples N` 指定丢弃样本数，或 `--heights keep` 保留高度。
输出目录必须尚不存在。扩展名不改变输入的解释方式。
若输入旁没有 `run.json`，完成状态记为 **unverified**；语法完整的文件并不能证明分析已结束。
若有运行记录，则检查完成状态、样本数量及每个采样间隔。

当前读取器支持 Rove 输出的有根、二叉、超度量树，要求末端同期、含翻译表，
且 `STATE_` 标识符严格递增。混合分类单元、缺少结尾 `End;`、不完整速率注释或不支持的树形均被拒绝。
不支持化石末端日期、多分叉和任意外部 NEXUS 块。
限制为 4,096 个分类单元、512 层嵌套、每条语句 2 MiB，以及一百万个不同支系或支系键
估计占用 128 MiB（先到者为准）。该估计不是进程内存的硬上限。
四次流式遍历只保留支系计数和目标树统计，不在内存中保存全部解析树。

[验证与 Apple silicon 测量](https://github.com/getcortexlabs/rove/blob/main/benchmarks/tree-summary/2026-10-29/README.md)
记录了与固定版本 TreeAnnotator 和外部 NEXUS 读取器的比较。
