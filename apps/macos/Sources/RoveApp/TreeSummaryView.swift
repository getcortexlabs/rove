import SwiftUI

struct TreeSummaryView: View {
  @Bindable var model: TreeSummaryModel
  let strings: AppStrings
  var onOpen: (URL) -> Void
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      ScrollView {
        content.frame(maxWidth: .infinity, alignment: .leading).padding(.bottom, 6)
      }.frame(maxHeight: 560).scrollBounceBehavior(.basedOnSize)
      HStack {
        Button(strings("Close")) { dismiss() }.keyboardShortcut(.cancelAction).disabled(model.busy)
        Spacer()
        Button(strings("Create tree summary"), action: model.start)
          .buttonStyle(LabButtonStyle(prominent: true)).disabled(!model.valid || model.busy)
          .keyboardShortcut(.defaultAction)
      }.padding(.top, 18)
    }.padding(28).frame(width: 620).foregroundStyle(Lab.ink).background(Lab.background)
      .preferredColorScheme(.dark).tint(Lab.accent)
      .task { await model.load() }.onDisappear { model.cancel() }
  }

  private var content: some View {
    VStack(alignment: .leading, spacing: 20) {
      Eyebrow(text: strings("From samples to a tree"), color: Lab.ice)
      Text(strings("Find the shared branches.")).font(Lab.display(28))
      if case .file(let url) = model.source {
        HStack(spacing: 8) {
          Image(systemName: "doc.text").foregroundStyle(Lab.ice)
          Text(url.lastPathComponent).font(.system(size: 12, design: .monospaced))
            .lineLimit(1).truncationMode(.middle)
          Text(strings("Trees from another program")).font(.system(size: 11))
            .foregroundStyle(Lab.muted)
        }
      }
      Text(
        strings(
          "Build an annotated MCC tree from the sampled trees. Your original files stay unchanged.")
      )
      .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
      HStack(alignment: .top, spacing: 24) {
        VStack(alignment: .leading, spacing: 8) {
          Text(strings("Burn-in")).font(.system(size: 12, weight: .medium))
          HStack(spacing: 8) {
            TextField(strings("Burn-in"), text: $model.burnin).textFieldStyle(.roundedBorder)
              .frame(width: 90)
            Picker(strings("Burn-in unit"), selection: $model.burninUnit) {
              Text(strings("Percent")).tag("percent")
              Text(strings("Samples")).tag("samples")
            }.pickerStyle(.segmented).labelsHidden().frame(width: 170)
          }
          Text(
            strings(
              model.burninUnit == "samples"
                ? "Discard this many of the first samples, counting the state before the first step. Choose it after checking the trace."
                : "Discard this percentage of the first samples, rounded down. Choose it after checking the trace."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).fixedSize(
            horizontal: false, vertical: true)
        }.frame(maxWidth: .infinity, alignment: .leading)
        VStack(alignment: .leading, spacing: 8) {
          Text(strings("Node heights")).font(.system(size: 12, weight: .medium))
          Picker(strings("Node heights"), selection: $model.heights) {
            Text(strings("Common ancestor mean")).tag("ca")
            Text(strings("Keep selected tree")).tag("keep")
          }.labelsHidden()
          Text(
            strings(
              "Common ancestor means use every retained tree. Keeping heights preserves those of the selected MCC tree."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).fixedSize(
            horizontal: false, vertical: true)
        }.frame(maxWidth: .infinity, alignment: .leading)
      }.disabled(model.busy)
      Text(
        strings(
          "MCC selects a sampled tree by clade credibility. Support is the frequency of a clade after burn-in; imposed groups are marked. Ranges are minima and maxima, not credible intervals."
        )
      )
      .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
      if let error = model.error {
        DiagnosticMessage(title: strings(error), details: model.details, strings: strings)
      }
      if model.busy {
        HStack(spacing: 10) {
          ProgressView().controlSize(.small)
          Text(strings("Reading trees and calculating the summary…")).font(.system(size: 12))
          Spacer()
          Button(strings("Cancel"), action: model.cancel)
        }
      }
      if !model.summaries.isEmpty {
        VStack(alignment: .leading, spacing: 12) {
          Eyebrow(text: strings("Summaries"))
          VStack(alignment: .leading, spacing: 12) {
            ForEach(model.summaries.reversed()) { summary in
              VStack(alignment: .leading, spacing: 12) {
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
                Button {
                  onOpen(summary.directory)
                  dismiss()
                } label: {
                  Label(strings("View tree"), systemImage: "point.3.connected.trianglepath.dotted")
                }.buttonStyle(LabButtonStyle(compact: true)).disabled(model.busy)
                HStack(spacing: 10) {
                  file("Annotated tree", "mcc.tree", summary.directory)
                  file("Raw trees (.trees)", "samples.trees", summary.directory)
                  file("Summary record", "summary.json", summary.directory)
                }
              }.padding(16).frame(maxWidth: .infinity, alignment: .leading)
                .background(Lab.surface, in: RoundedRectangle(cornerRadius: 12))
            }
          }
        }
      }
      Text(
        strings(
          "A summary and high support do not establish convergence. Check independent runs and sampling diagnostics before interpreting the tree."
        )
      )
      .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
    }
  }

  private func file(_ title: String, _ name: String, _ directory: URL) -> some View {
    Button {
      NSWorkspace.shared.activateFileViewerSelecting([directory.appendingPathComponent(name)])
    } label: {
      Text(strings(title))
    }.buttonStyle(LabButtonStyle(compact: true))
  }
}
