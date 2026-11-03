import SwiftUI

struct RunAssessmentView: View {
  @Bindable var workspace: Workspace
  let entry: HistoryEntry
  @State private var model: RunAssessmentModel?
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 16) {
        Eyebrow(text: strings("Evidence review"), color: Lab.lilac)
        Text(strings("What does this run establish?")).font(Lab.display(25))
        Text(
          strings(
            "Review completion, sampling and available reports separately. This snapshot does not certify convergence or model adequacy."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary).fixedSize(
          horizontal: false, vertical: true)
        if let model { content(model) }
      }
    }.task(id: entry.id) {
      let value = workspace.runAssessment(for: entry)
      model = value
      await value.load()
    }
  }

  @ViewBuilder private func content(_ model: RunAssessmentModel) -> some View {
    @Bindable var model = model
    HStack {
      Text(strings("Burn-in (%)"))
      TextField(strings("Burn-in (%)"), text: $model.options.burnin).textFieldStyle(.roundedBorder)
        .frame(width: 70)
      Button(strings("Refresh evidence review")) { model.start() }
        .buttonStyle(LabButtonStyle(prominent: true)).disabled(
          !model.options.valid || model.busy || workspace.isRunning)
      if model.busy {
        ProgressView().controlSize(.small)
        Button(strings("Cancel")) { model.cancel() }.buttonStyle(LabButtonStyle(compact: true))
      }
    }.font(.system(size: 11))
    if let report = model.report {
      Text(
        Date(timeIntervalSince1970: report.createdUnixSeconds).formatted(
          .dateTime.day().month().year().hour().minute().locale(strings.locale))
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted)
      ForEach(report.sections) { section in
        VStack(alignment: .leading, spacing: 6) {
          HStack {
            Text(strings(Self.title(section.id))).font(.system(size: 12, weight: .medium))
            Spacer()
            Text(strings(Self.status(section.status))).foregroundStyle(
              section.status == "passed" ? Lab.accent : Lab.secondary)
          }
          Text(strings(section.message)).font(.system(size: 10)).foregroundStyle(Lab.secondary)
            .fixedSize(horizontal: false, vertical: true)
        }
      }
      Text(
        strings(
          "Matching tree reports: %@ · Predictive reports: %@", String(report.treeReports.count),
          String(report.predictiveReports.count))
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted)
      Text(strings(Self.inputStatus(report.originalMatrixStatus))).font(.system(size: 10))
        .foregroundStyle(Lab.secondary)
      if let directory = model.reportDirectory {
        Button(strings("Review record")) {
          NSWorkspace.shared.open(directory.appendingPathComponent("assessment.json"))
        }
        .buttonStyle(LabButtonStyle(compact: true))
      }
    }
    Divider().overlay(Lab.border)
    Toggle(strings("Include the original matrix in the export"), isOn: $model.options.includeMatrix)
      .font(.system(size: 11)).disabled(model.busy)
    Text(
      strings(
        "The local package includes settings, seeds, build identities, samples, diagnostics and derived reports with SHA-256 fingerprints. Recorded paths and taxon names are preserved. Exact resumption still requires the original executable and a valid checkpoint."
      )
    )
    .font(.system(size: 10)).foregroundStyle(Lab.secondary).fixedSize(
      horizontal: false, vertical: true)
    Button(strings("Export reproducibility package…")) {
      let panel = NSOpenPanel()
      panel.canChooseFiles = false
      panel.canChooseDirectories = true
      panel.canCreateDirectories = true
      panel.allowsMultipleSelection = false
      panel.prompt = strings("Choose export folder")
      guard panel.runModal() == .OK, let folder = panel.url else { return }
      model.start(exportTo: folder.appendingPathComponent("rove-export-\(UUID().uuidString)"))
    }.buttonStyle(LabButtonStyle(compact: true)).disabled(
      !model.options.valid || model.busy || workspace.isRunning)
    if let exported = model.exported {
      Button(strings("Open exported package")) { NSWorkspace.shared.open(exported) }.buttonStyle(
        LabButtonStyle(compact: true))
    }
    if let error = model.error {
      DiagnosticMessage(title: strings(error), details: model.details, strings: strings)
    }
  }

  static func title(_ id: String) -> String {
    switch id {
    case "completion": "Run completion"
    case "sampling": "Scalar sampling"
    case "agreement": "Independent-chain agreement"
    case "trees": "Tree evidence"
    default: "Predictive evidence"
    }
  }
  static func status(_ value: String) -> String {
    switch value {
    case "passed": "Criteria met"
    case "attention": "Needs attention"
    case "review_required": "Interpretation required"
    default: "Evidence unavailable"
    }
  }
  static func inputStatus(_ value: String) -> String {
    switch value {
    case "matches": "The original matrix matches the recorded fingerprint."
    case "changed": "The original matrix has changed. Matrix inclusion will be refused."
    default: "The original matrix is unavailable. Records can still be exported without it."
    }
  }
}
