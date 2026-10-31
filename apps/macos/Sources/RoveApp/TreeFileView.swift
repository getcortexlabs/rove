import SwiftUI

/// A tree file from another program and the summaries Rove made of it.
struct TreeFileDetail: View {
  @Environment(\.openWindow) private var openWindow
  @Bindable var workspace: Workspace
  let entry: HistoryEntry
  let source: TreeSource
  @State private var summaries: [SavedSummary] = []
  private var strings: AppStrings { workspace.strings }
  private var file: URL { URL(fileURLWithPath: source.path) }
  private var available: Bool { FileManager.default.isReadableFile(atPath: source.path) }

  var body: some View {
    VStack(alignment: .leading, spacing: 22) {
      VStack(alignment: .leading, spacing: 10) {
        Eyebrow(text: strings("Tree file"), color: Lab.accent)
        Text(entry.name).font(Lab.display(30)).textSelection(.enabled)
        Text(
          source.createdAt.formatted(
            .dateTime.day().month(.abbreviated).year().hour().minute().locale(strings.locale))
        )
        .font(.system(size: 12)).foregroundStyle(Lab.secondary)
      }
      LabPanel {
        VStack(alignment: .leading, spacing: 14) {
          Text(
            strings(
              "Rove did not run this sample here, so no outgroup is known. A summary and high node support do not establish convergence."
            )
          )
          .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
          Divider().overlay(Lab.border)
          VStack(alignment: .leading, spacing: 8) {
            Text(source.path)
            Text("SHA-256: \(source.sha256)")
          }.font(.system(size: 10, design: .monospaced)).foregroundStyle(Lab.secondary)
            .textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading)
          if !available {
            InlineMessage(
              text: strings(
                "The tree file is no longer at its recorded path. Saved summaries stay available."),
              isError: false)
          }
          ViewThatFits(in: .horizontal) {
            HStack(spacing: 8) { buttons }
            VStack(alignment: .leading, spacing: 8) { buttons }
          }
        }
      }
      LabPanel {
        VStack(alignment: .leading, spacing: 14) {
          Eyebrow(text: strings("Summaries"))
          if summaries.isEmpty {
            Text(strings("No summary yet. Summarize the file to build an annotated MCC tree."))
              .font(.system(size: 12)).foregroundStyle(Lab.secondary)
          }
          ForEach(summaries.reversed()) { summary in
            VStack(alignment: .leading, spacing: 10) {
              Text(
                strings(
                  "%@ trees retained · %@ discarded",
                  strings.number(summary.record.retainedSamples),
                  strings.number(summary.record.discardedSamples))
              )
              .font(.system(size: 13, weight: .semibold))
              Text(
                "MCC · \(strings(summary.record.heightPolicy == "ca" ? "Common ancestor mean" : "Keep selected tree")) · STATE_\(String(summary.record.selectedState))"
              )
              .font(.system(size: 11)).foregroundStyle(Lab.secondary)
              HStack(spacing: 10) {
                Button {
                  workspace.openTree(summary.directory, run: entry.directory)
                  openWindow(id: "tree-explorer")
                } label: {
                  Label(strings("View tree"), systemImage: "point.3.connected.trianglepath.dotted")
                }.buttonStyle(LabButtonStyle(compact: true))
                reveal("Annotated tree", summary.directory.appendingPathComponent("mcc.tree"))
                reveal("Summary record", summary.directory.appendingPathComponent("summary.json"))
              }
            }.padding(16).frame(maxWidth: .infinity, alignment: .leading)
              .background(Lab.surface, in: RoundedRectangle(cornerRadius: 12))
          }
        }
      }
    }
    .task(id: "\(entry.id)-\(workspace.treeSummary == nil)") {
      let directory = entry.directory
      summaries = await Task.detached { SavedSummary.list(in: directory) }.value
    }
  }

  @ViewBuilder private var buttons: some View {
    Button {
      workspace.openTreeFile(file)
    } label: {
      Label(strings("Summarize trees"), systemImage: "point.3.connected.trianglepath.dotted")
    }.buttonStyle(LabButtonStyle(prominent: true))
      .disabled(!available || workspace.isRunning || workspace.loading)
    Button {
      NSWorkspace.shared.activateFileViewerSelecting([file])
    } label: {
      Label(strings("Reveal tree file"), systemImage: "doc.text")
    }.buttonStyle(LabButtonStyle(compact: true)).disabled(!available)
    Button {
      NSWorkspace.shared.open(entry.directory)
    } label: {
      Label(strings("Open folder"), systemImage: "folder")
    }.buttonStyle(LabButtonStyle(compact: true))
  }

  private func reveal(_ label: String, _ url: URL) -> some View {
    Button(strings(label)) { NSWorkspace.shared.activateFileViewerSelecting([url]) }
      .buttonStyle(LabButtonStyle(compact: true))
  }
}
