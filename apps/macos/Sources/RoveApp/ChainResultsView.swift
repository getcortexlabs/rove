import SwiftUI

struct ChainResultsView: View {
  @Environment(\.openWindow) private var openWindow
  @Bindable var workspace: Workspace
  let directory: URL
  let settings: AnalysisSettings
  let completed: Bool
  @State private var report: ChainComparison?
  @State private var error: String?
  @State private var selectedChain = 1
  private var strings: AppStrings { workspace.strings }
  private var chainDirectory: URL {
    directory.appendingPathComponent(String(format: "results/chain-%02d", selectedChain))
  }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 20) {
        HStack {
          VStack(alignment: .leading, spacing: 7) {
            Eyebrow(text: strings("Compare the paths"), color: Lab.lilac)
            Text(strings("Do the chains agree?")).font(Lab.display(25))
          }
          Spacer()
          Image(systemName: "point.3.connected.trianglepath.dotted")
            .font(.system(size: 32, weight: .light)).foregroundStyle(Lab.lilac)
        }
        if !completed {
          Text(
            strings(
              "The chain comparison will be available when this group finishes. You can inspect each chain's partial samples below."
            )
          )
          .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
        } else if let report {
          ComparisonReportView(
            report: report,
            labels: (1...settings.chains).map { strings("Chain %@", String($0)) },
            strings: strings)
        } else if let error {
          DiagnosticMessage(
            title: strings("The chain comparison is unavailable."), details: error, strings: strings
          )
        } else {
          ProgressView().controlSize(.small)
        }
        Divider().overlay(Lab.border)
        HStack(spacing: 12) {
          Picker(strings("Chain outputs"), selection: $selectedChain) {
            ForEach(1...settings.chains, id: \.self) { chain in
              Text(strings("Chain %@", String(chain))).tag(chain)
            }
          }.labelsHidden().frame(width: 155)
          Text(
            strings("Seed %@", String((UInt64(settings.seed) ?? 0) &+ UInt64(selectedChain - 1)))
          )
          .font(.system(size: 10, design: .monospaced)).foregroundStyle(Lab.muted)
          Spacer()
          Button(strings("Diagnostics")) {
            workspace.openDiagnostics(in: chainDirectory)
            openWindow(id: "trace-diagnostics")
          }.buttonStyle(LabButtonStyle(compact: true))
          Button(strings("Summarize trees")) { workspace.summarizeTrees(in: chainDirectory) }
            .buttonStyle(LabButtonStyle(compact: true)).disabled(!completed)
        }
        HStack {
          Button(strings("Open chain folder")) { NSWorkspace.shared.open(chainDirectory) }
            .buttonStyle(LabButtonStyle(compact: true))
          Button(strings("Comparison file")) {
            NSWorkspace.shared.activateFileViewerSelecting([
              directory.appendingPathComponent("results/comparison.json")
            ])
          }.buttonStyle(LabButtonStyle(compact: true)).disabled(report == nil)
        }
      }
    }.task(id: [directory.path, String(completed)]) {
      report = nil
      error = nil
      guard completed else { return }
      let path = directory.appendingPathComponent("results/comparison.json")
      do {
        let value = try await Task.detached { try ChainComparison.read(path) }.value
        guard value.chainCount == settings.chains else { throw RunFailure.invalidCompletion }
        report = value
      } catch { self.error = error.localizedDescription }
    }
  }

}
