import SwiftUI

struct RunView: View {
  @Bindable var workspace: Workspace
  let run: AnalysisRun
  @State private var confirmingCancellation = false
  private var strings: AppStrings { workspace.strings }
  private var total: Int { run.request.draft.settings.steps }
  private var step: UInt64 { run.progress?.step ?? 0 }
  private var statusTitle: String {
    switch run.status {
    case .preparing: "Preparing your exploration…"
    case .running: "Following the branches."
    case .cancelling: "Closing this exploration…"
    case .completed: "The run is complete."
    case .cancelled: "Stopped along the way."
    case .failed: "This run needs a closer look."
    }
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 24) {
      HStack(alignment: .center, spacing: 24) {
        VStack(alignment: .leading, spacing: 12) {
          Eyebrow(text: strings("04 / Explore"), color: Lab.accent)
          Text(strings(statusTitle)).font(Lab.display(34)).tracking(-1)
          Text(URL(fileURLWithPath: run.request.draft.matrixPath).lastPathComponent)
            .font(.system(size: 12)).foregroundStyle(Lab.secondary)
        }
        Spacer()
        if let image = Lab.explorerImage {
          Image(nsImage: image).resizable().scaledToFit().frame(width: 140, height: 140)
            .accessibilityHidden(true)
        }
      }.padding(.top, 8)
      LabPanel {
        VStack(alignment: .leading, spacing: 20) {
          HStack(alignment: .firstTextBaseline) {
            Text(strings("%@ of %@ steps", strings.number(Int(step)), strings.number(total)))
              .font(.system(size: 15, weight: .medium, design: .monospaced)).monospacedDigit()
            Spacer()
            Text(strings(run.status.rawValue.capitalized)).font(.system(size: 11, weight: .medium))
              .foregroundStyle(Lab.accent).padding(.horizontal, 10).padding(.vertical, 6)
              .background(Lab.accent.opacity(0.08), in: Capsule())
          }
          if run.status == .preparing {
            ProgressView().progressViewStyle(.linear).accessibilityLabel(strings("Preparing"))
          } else {
            ProgressView(value: Double(step), total: Double(total))
              .progressViewStyle(RunProgressStyle())
              .accessibilityLabel(strings("Analysis progress"))
              .accessibilityValue(
                strings("%@ of %@ steps", strings.number(Int(step)), strings.number(total)))
          }
          HStack(spacing: 36) {
            if let ended = run.ended {
              metric("Elapsed", duration(ended.timeIntervalSince(run.began)))
            } else {
              TimelineView(.periodic(from: run.began, by: 1)) { context in
                metric("Elapsed", duration(context.date.timeIntervalSince(run.began)))
              }
            }
            metric("Random seed", run.request.draft.settings.seed)
            metric("Groups constrained", strings.number(run.request.draft.constraints.count))
            Spacer()
          }
          DisclosureGroup(strings("Applied analysis setup")) {
            ConstraintSummary(
              outgroup: run.request.draft.outgroup, taxa: run.request.draft.taxa,
              groups: run.request.draft.constraints, strings: strings
            ).padding(.top, 10)
          }.font(.system(size: 12))
          Text(
            strings(
              "Progress counts completed MCMC steps. It does not measure convergence or confidence."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(4)
        }
      }
      if let result = run.result, let error = result.error {
        DiagnosticMessage(title: strings(error), details: result.details, strings: strings)
      }
      if run.status == .completed {
        InlineMessage(
          text: strings(
            "The requested steps finished and the output files were verified. Assess independent runs, sampling diagnostics and tree support before interpreting the result."
          ))
      } else if run.status == .cancelled || run.status == .cancelling {
        InlineMessage(
          text: strings(
            "Partial files are kept in the run folder. They are not a completed analysis or a checkpoint. A new run starts from the beginning."
          ))
      } else if run.status == .failed {
        InlineMessage(
          text: strings(
            "Any files in this folder are partial. Inspect the run record and engine log before using them."
          ), isError: true)
      }
      LabPanel {
        VStack(alignment: .leading, spacing: 14) {
          HStack {
            Eyebrow(text: strings("Run folder"))
            Spacer()
            Button {
              NSWorkspace.shared.open(run.request.directory)
            } label: {
              Label(strings("Open folder"), systemImage: "folder")
            }.buttonStyle(LabButtonStyle(compact: true))
              .disabled(!FileManager.default.fileExists(atPath: run.request.directory.path))
          }
          Text(run.request.directory.path).font(.system(size: 11, design: .monospaced))
            .foregroundStyle(Lab.secondary).textSelection(.enabled).fixedSize(
              horizontal: false, vertical: true)
          if !run.isActive {
            HStack(spacing: 12) {
              output("Trace", "trace.log")
              output("Trees", "trees.nexus")
              output("Run record", "run.json")
            }
          }
        }
      }
      HStack {
        if run.isActive {
          Button {
            confirmingCancellation = true
          } label: {
            Label(strings("Cancel analysis"), systemImage: "stop.circle")
          }.buttonStyle(LabButtonStyle(compact: true)).keyboardShortcut(".", modifiers: .command)
            .disabled(run.status == .cancelling)
        } else {
          Button(action: workspace.dismissRun) {
            Label(strings("Back to setup"), systemImage: "arrow.left")
          }.buttonStyle(LabButtonStyle(prominent: true))
        }
        Spacer()
        if run.status == .completed {
          Button(strings("Summarize trees")) { workspace.summarizeTrees(in: run.request.directory) }
            .buttonStyle(LabButtonStyle(compact: true))
        }
        Button(action: workspace.openHistory) {
          Label(strings("History"), systemImage: "clock.arrow.circlepath")
        }.buttonStyle(LabButtonStyle(compact: true))
      }
    }
    .confirmationDialog(
      strings("Stop this analysis?"), isPresented: $confirmingCancellation,
      titleVisibility: .visible
    ) {
      Button(strings("Stop and keep partial files"), role: .destructive) { run.cancel() }
      Button(strings("Keep running"), role: .cancel) {}
    } message: {
      Text(
        strings(
          "Rove will close the engine and keep partial output. Resuming a run is not available yet."
        ))
    }
  }

  private func metric(_ label: String, _ value: String) -> some View {
    VStack(alignment: .leading, spacing: 8) {
      Eyebrow(text: strings(label))
      Text(value).font(.system(size: 16, weight: .medium, design: .monospaced)).monospacedDigit()
    }
  }

  private func duration(_ seconds: TimeInterval) -> String {
    let total = max(0, Int(seconds))
    return String(format: "%02d:%02d:%02d", total / 3600, (total / 60) % 60, total % 60)
  }

  private func output(_ title: String, _ filename: String) -> some View {
    let url = run.request.output.appendingPathComponent(filename)
    return Button {
      NSWorkspace.shared.activateFileViewerSelecting([url])
    } label: {
      Label(strings(title), systemImage: "doc.text")
    }.buttonStyle(LabButtonStyle(compact: true))
      .disabled(!FileManager.default.fileExists(atPath: url.path))
  }
}

private struct RunProgressStyle: ProgressViewStyle {
  func makeBody(configuration: Configuration) -> some View {
    GeometryReader { geometry in
      ZStack(alignment: .leading) {
        Capsule().fill(Lab.border)
        Capsule().fill(
          LinearGradient(colors: [Lab.lilac, Lab.ice], startPoint: .leading, endPoint: .trailing)
        )
        .frame(width: geometry.size.width * (configuration.fractionCompleted ?? 0))
      }
    }.frame(height: 6)
  }
}
