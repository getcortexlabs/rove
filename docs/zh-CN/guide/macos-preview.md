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

## 从命令行检查

```sh
target/release/rove inspect fixtures/reference/20x50/matrix.nex
```

命令输出带版本号的 JSON，包含计数、有限的单元格预览、输入哈希，以及分别
说明采样器和 Mkv 适用性的错误。只有该解释合适时才加上 `--gaps missing`。
读取矩阵不会写入分析结果。
