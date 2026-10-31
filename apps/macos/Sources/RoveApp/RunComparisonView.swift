import SwiftUI

/// Compares a completed run with other compatible runs from History.
struct RunComparisonView: View {
  @Bindable var workspace: Workspace
  let entry: HistoryEntry
  @State private var model: RunComparisonModel?
  @State private var shown: String?
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 18) {
        HStack {
          VStack(alignment: .leading, spacing: 7) {
            Eyebrow(text: strings("Compare with other runs"), color: Lab.ice)
            Text(strings("Do independent runs agree?")).font(Lab.display(25))
          }
          Spacer()
          Image(systemName: "arrow.triangle.branch")
            .font(.system(size: 30, weight: .light)).foregroundStyle(Lab.ice)
        }
        Text(
          strings(
            "Runs with the same matrix, model and constraints and different seeds can be compared. Agreement between runs is not convergence of either."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(4)
        if let model {
          content(model)
            .onChange(of: model.comparisons.first?.id, initial: true) { _, newest in
              shown = newest
            }
        } else {
          ProgressView().controlSize(.small)
        }
      }
    }
    .task(id: entry.id) {
      let value = workspace.runComparison(for: entry)
      model = value
      shown = nil
      await value.load()
    }
  }

  @ViewBuilder private func content(_ model: RunComparisonModel) -> some View {
    @Bindable var model = model
    let candidates = model.candidates(in: workspace.history.entries)
    if candidates.isEmpty {
      Text(strings("No other completed run matches this setup."))
        .font(.system(size: 12)).foregroundStyle(Lab.secondary)
    } else {
      VStack(alignment: .leading, spacing: 8) {
        ForEach(candidates) { candidate in
          Toggle(isOn: binding(model, candidate.id)) {
            HStack(spacing: 10) {
              Text(candidate.directory.lastPathComponent).lineLimit(1).truncationMode(.middle)
              Text(describe(candidate)).foregroundStyle(Lab.muted)
            }.font(.system(size: 11))
          }.toggleStyle(.checkbox).disabled(model.busy || !model.canSelect(candidate, in: candidates))
        }
        if candidates.contains(where: { !model.canSelect($0, in: candidates) }) {
          Text(strings("Runs with seeds that overlap a selected run, or beyond 16 chains in total, are unavailable."))
            .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
        }
      }
      HStack(spacing: 12) {
        Text(strings("Burn-in (%)")).font(.system(size: 12, weight: .medium))
        TextField(strings("Burn-in (%)"), text: $model.burnin).textFieldStyle(.roundedBorder)
          .frame(width: 70).disabled(model.busy)
        Button(strings("Compare runs")) { model.start(with: candidates) }
          .buttonStyle(LabButtonStyle(prominent: true))
          .disabled(model.busy || !model.valid || model.selected.isEmpty)
        if model.busy {
          ProgressView().controlSize(.small)
          Text(strings("Comparing runs…")).font(.system(size: 11)).foregroundStyle(Lab.secondary)
          Button(strings("Cancel")) { model.cancel() }.buttonStyle(LabButtonStyle(compact: true))
        }
      }
    }
    if let error = model.error {
      DiagnosticMessage(title: strings(error), details: model.details, strings: strings)
    }
    if !model.comparisons.isEmpty {
      Divider().overlay(Lab.border)
      VStack(alignment: .leading, spacing: 10) {
        Eyebrow(text: strings("Saved comparisons"))
        ForEach(model.comparisons) { comparison in
          DisclosureGroup(
            isExpanded: Binding(
              get: { shown == comparison.id }, set: { shown = $0 ? comparison.id : nil })
          ) {
            VStack(alignment: .leading, spacing: 14) {
              ComparisonReportView(
                report: comparison.report, labels: labels(comparison), strings: strings)
              ForEach(comparison.sources, id: \.self) { source in
                Text(source).font(.system(size: 10, design: .monospaced))
                  .foregroundStyle(Lab.secondary).textSelection(.enabled)
              }
              Button(strings("Comparison file")) {
                NSWorkspace.shared.activateFileViewerSelecting([comparison.file])
              }.buttonStyle(LabButtonStyle(compact: true))
            }.padding(.top, 12)
          } label: {
            HStack(spacing: 10) {
              Text(
                strings(
                  "%@ chains · %@%% burn-in", String(comparison.report.chainCount),
                  String(comparison.report.burninPercent)))
              Text(date(comparison.createdAt)).foregroundStyle(Lab.muted)
            }.font(.system(size: 11))
          }
        }
      }
    }
  }

  private func binding(_ model: RunComparisonModel, _ id: String) -> Binding<Bool> {
    Binding(
      get: { model.selected.contains(id) },
      set: { if $0 { model.selected.insert(id) } else { model.selected.remove(id) } })
  }

  private func describe(_ candidate: HistoryEntry) -> String {
    guard let draft = candidate.draft else { return "" }
    let seed = strings("Seed %@", draft.settings.seed)
    return draft.settings.chains > 1
      ? "\(seed) · \(strings("%@ chains", String(draft.settings.chains)))" : seed
  }

  /// One label per compared chain, in the engine's order: this run first.
  private func labels(_ comparison: SavedComparison) -> [String] {
    zip(comparison.seeds, comparison.sources).map { seed, source in
      seed.map { strings("Seed %@", String($0)) } ?? URL(fileURLWithPath: source).lastPathComponent
    }
  }

  private func date(_ value: Date?) -> String {
    value?.formatted(
      .dateTime.day().month(.abbreviated).year().hour().minute().locale(strings.locale))
      ?? strings("Date unavailable")
  }
}
