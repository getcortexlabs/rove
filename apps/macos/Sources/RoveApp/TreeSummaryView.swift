import SwiftUI

struct TreeSummaryView: View {
  @Bindable var model: TreeSummaryModel
  let strings: AppStrings
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    VStack(alignment: .leading, spacing: 20) {
      Eyebrow(text: strings("From samples to a tree"), color: Lab.ice)
      Text(strings("Find the shared branches.")).font(Lab.display(28))
      Text(
        strings(
          "Build an annotated MCC tree from the sampled trees. Your original files stay unchanged.")
      )
      .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
      HStack(alignment: .top, spacing: 24) {
        VStack(alignment: .leading, spacing: 8) {
          Text(strings("Burn-in (%)")).font(.system(size: 12, weight: .medium))
          TextField(strings("Burn-in (%)"), text: $model.burnin).textFieldStyle(.roundedBorder)
            .frame(width: 90)
          Text(
            strings(
              "Discard this percentage of the first samples, rounded down. Choose it after checking the trace."
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
        ScrollView {
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
                HStack(spacing: 10) {
                  file("Annotated tree", "mcc.tree", summary.directory)
                  file("Raw trees (.trees)", "samples.trees", summary.directory)
                  file("Summary record", "summary.json", summary.directory)
                }
              }.padding(16).frame(maxWidth: .infinity, alignment: .leading)
                .background(Lab.surface, in: RoundedRectangle(cornerRadius: 12))
            }
          }
        }.frame(maxHeight: 210)
      }
      Text(
        strings(
          "A summary and high support do not establish convergence. Check independent runs and sampling diagnostics before interpreting the tree."
        )
      )
      .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
      HStack {
        Button(strings("Close")) { dismiss() }.keyboardShortcut(.cancelAction).disabled(model.busy)
        Spacer()
        Button(strings("Create tree summary"), action: model.start)
          .buttonStyle(LabButtonStyle(prominent: true)).disabled(!model.valid || model.busy)
          .keyboardShortcut(.defaultAction)
      }
    }.padding(28).frame(width: 620).foregroundStyle(Lab.ink).background(Lab.background)
      .preferredColorScheme(.dark).tint(Lab.accent)
      .task { await model.load() }.onDisappear { model.cancel() }
  }

  private func file(_ title: String, _ name: String, _ directory: URL) -> some View {
    Button {
      NSWorkspace.shared.activateFileViewerSelecting([directory.appendingPathComponent(name)])
    } label: {
      Text(strings(title))
    }.buttonStyle(LabButtonStyle(compact: true))
  }
}
