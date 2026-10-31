import SwiftUI

/// Posterior predictive checks of a completed run, saved beside it.
struct PredictiveCheckView: View {
  @Bindable var workspace: Workspace
  let entry: HistoryEntry
  @State private var model: PredictiveCheckModel?
  @State private var shown: String?
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 18) {
        HStack {
          VStack(alignment: .leading, spacing: 7) {
            Eyebrow(text: strings("Posterior predictive check"), color: Lab.lilac)
            Text(strings("Can the model reproduce the matrix?")).font(Lab.display(25))
          }
          Spacer()
          Image(systemName: "square.on.square.dashed")
            .font(.system(size: 30, weight: .light)).foregroundStyle(Lab.lilac)
        }
        Text(
          strings(
            "Replicate matrices are simulated from posterior trees and parameters and scored against the observed matrix on the same sampled tree. Small tail fractions mark a discrepancy to investigate; no threshold decides adequacy."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(4)
        if let model {
          content(model)
            .onChange(of: model.predictions.first?.id, initial: true) { _, newest in
              shown = newest
            }
        } else {
          ProgressView().controlSize(.small)
        }
      }
    }
    .task(id: entry.id) {
      let value = workspace.predictiveCheck(for: entry)
      model = value
      shown = nil
      await value.load()
    }
  }

  @ViewBuilder private func content(_ model: PredictiveCheckModel) -> some View {
    @Bindable var model = model
    HStack(alignment: .top, spacing: 18) {
      field("Burn-in (%)", text: $model.burnin, width: 70)
      field("Replicates", text: $model.replicates, width: 80)
      field("Seed", text: $model.seed, width: 130)
      if model.chains > 1 {
        VStack(alignment: .leading, spacing: 6) {
          Text(strings("Source chain")).font(.system(size: 11, weight: .medium))
          Picker(strings("Source chain"), selection: $model.chain) {
            ForEach(1...model.chains, id: \.self) { Text(strings("Chain %@", String($0))).tag($0) }
          }.labelsHidden().frame(width: 130)
        }
      }
    }.disabled(model.busy)
    Text(
      strings(
        "Each replicate draws a retained posterior sample with replacement, so repeated draws are expected. Chains are not pooled: a group is checked one chain at a time."
      )
    )
    .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
    HStack(spacing: 12) {
      Button(strings("Run predictive check")) { model.start() }
        .buttonStyle(LabButtonStyle(prominent: true)).disabled(model.busy || !model.valid)
      if model.busy {
        ProgressView(value: Double(model.completed), total: Double(max(model.total, 1)))
          .frame(width: 140)
        Text(
          strings(
            "%@ of %@ replicates", strings.number(model.completed), strings.number(model.total))
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary)
        Button(strings("Cancel")) { model.cancel() }.buttonStyle(LabButtonStyle(compact: true))
      }
    }
    if let error = model.error {
      DiagnosticMessage(title: strings(error), details: model.details, strings: strings)
    }
    if !model.predictions.isEmpty {
      Divider().overlay(Lab.border)
      VStack(alignment: .leading, spacing: 10) {
        Eyebrow(text: strings("Saved checks"))
        ForEach(model.predictions) { prediction in
          DisclosureGroup(
            isExpanded: Binding(
              get: { shown == prediction.id }, set: { shown = $0 ? prediction.id : nil })
          ) {
            PredictiveReportView(prediction: prediction, strings: strings).padding(.top, 12)
          } label: {
            HStack(spacing: 10) {
              Text(label(prediction))
              Text(
                prediction.createdAt.formatted(
                  .dateTime.day().month(.abbreviated).year().hour().minute()
                    .locale(strings.locale))
              )
              .foregroundStyle(Lab.muted)
            }.font(.system(size: 11))
          }
        }
      }
    }
  }

  private func label(_ prediction: SavedPrediction) -> String {
    let report = prediction.report
    var text = strings(
      "%@ replicates · %@ of %@ samples · seed %@", strings.number(report.replicates),
      strings.number(report.source.retainedSamples), strings.number(report.source.totalSamples),
      String(report.seed))
    let source = URL(fileURLWithPath: report.source.directory).lastPathComponent
    if let chain = Int(source.dropFirst("chain-".count)), source.hasPrefix("chain-") {
      text += " · " + strings("Chain %@", String(chain))
    }
    return text
  }

  private func field(_ label: String, text: Binding<String>, width: CGFloat) -> some View {
    VStack(alignment: .leading, spacing: 6) {
      Text(strings(label)).font(.system(size: 11, weight: .medium))
      TextField(strings(label), text: text).textFieldStyle(.roundedBorder)
        .font(.system(size: 12, design: .monospaced)).frame(width: width).labelsHidden()
    }
  }
}

/// The matrix-wide tail fractions of one saved check.
struct PredictiveReportView: View {
  let prediction: SavedPrediction
  let strings: AppStrings

  var body: some View {
    let report = prediction.report
    VStack(alignment: .leading, spacing: 14) {
      Text(
        strings(
          "%@ posterior samples retained after discarding %@; the first retained state is %@. The reconstructed likelihood differed from the trace by at most %@.",
          strings.number(report.source.retainedSamples),
          strings.number(report.source.discardedSamples),
          String(report.source.firstRetainedState),
          difference(report.source.maxReconstructedLikelihoodDifference))
      )
      .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
      ScrollView(.horizontal) {
        Grid(alignment: .leading, horizontalSpacing: 22, verticalSpacing: 10) {
          GridRow {
            Text(strings("Statistic"))
            Text(strings("Valid pairs"))
            Text(strings("Lower tail"))
            Text(strings("Upper tail"))
            Text(strings("Ties"))
          }.font(.system(size: 10, weight: .semibold)).foregroundStyle(Lab.muted)
          Divider().gridCellColumns(5)
          ForEach(PredictiveReport.statistics, id: \.self) { name in
            if let tail = report.global[name] {
              GridRow {
                Text(strings(Self.label(name))).foregroundStyle(Lab.secondary)
                Text(pairs(tail))
                Text(fraction(tail.pLower, tail.lowerSimulationInterval95))
                Text(fraction(tail.pUpper, tail.upperSimulationInterval95))
                Text(strings.number(tail.ties))
              }.font(.system(size: 11, design: .monospaced)).monospacedDigit()
            }
          }
        }
      }.frame(maxWidth: .infinity, alignment: .leading)
      Text(
        strings(
          "A tail fraction is the share of valid pairs where the simulated statistic is at most (lower) or at least (upper) the observed one; ties count in both. Brackets hold 95% Wilson intervals for simulation error only, not MCMC error. Undefined consistency and retention pairs are excluded and counted."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
      HStack(spacing: 10) {
        Button(strings("Check record")) {
          NSWorkspace.shared.activateFileViewerSelecting([
            prediction.directory.appendingPathComponent("predictive.json")
          ])
        }
        Button(strings("Replicate matrices")) {
          NSWorkspace.shared.open(prediction.directory.appendingPathComponent("matrices"))
        }
        Button(strings("Open folder")) { NSWorkspace.shared.open(prediction.directory) }
      }.buttonStyle(LabButtonStyle(compact: true))
    }
  }

  static func label(_ statistic: String) -> String {
    switch statistic {
    case "parsimony_length": "Parsimony length"
    case "consistency_index": "Consistency index"
    case "retention_index": "Retention index"
    case "pattern_surprise": "Pattern surprise"
    default: "Informative characters"
    }
  }

  private func pairs(_ tail: PredictiveReport.Tail) -> String {
    guard tail.undefinedPairs > 0 else { return strings.number(tail.validPairs) }
    return strings(
      "%@ (%@ undefined)", strings.number(tail.validPairs), strings.number(tail.undefinedPairs))
  }

  private func fraction(_ value: Double?, _ interval: [Double]?) -> String {
    guard let value else { return "—" }
    let text = number(value)
    guard let interval, interval.count == 2 else { return text }
    return "\(text) [\(number(interval[0])), \(number(interval[1]))]"
  }

  private func number(_ value: Double) -> String {
    value.formatted(.number.precision(.fractionLength(3)).locale(strings.locale))
  }

  private func difference(_ value: Double) -> String {
    guard value != 0 else { return "0" }
    return value.formatted(
      .number.notation(.scientific).precision(.significantDigits(2)).locale(strings.locale))
  }
}
