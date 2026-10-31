import SwiftUI

/// Marginal-likelihood estimates of a completed run's model, and log Bayes
/// factors against estimates saved with other runs of the same matrix.
struct MarginalLikelihoodView: View {
  @Bindable var workspace: Workspace
  let entry: HistoryEntry
  @State private var model: MarginalLikelihoodModel?
  @State private var shown: String?
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 18) {
        HStack {
          VStack(alignment: .leading, spacing: 7) {
            Eyebrow(text: strings("Model comparison"), color: Lab.ice)
            Text(strings("Which model explains the matrix better?")).font(Lab.display(25))
          }
          Spacer()
          Image(systemName: "scalemass")
            .font(.system(size: 30, weight: .light)).foregroundStyle(Lab.ice)
        }
        Text(
          strings(
            "Power posteriors estimate the log marginal likelihood of this run's model: the probability of the matrix with every tree and parameter integrated out, priors included. The difference between two models is a log Bayes factor."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(4)
        if let model {
          content(model)
            .onChange(of: model.estimates.first?.id, initial: true) { _, newest in
              shown = newest
            }
        } else {
          ProgressView().controlSize(.small)
        }
      }
    }
    .task(id: entry.id) {
      let value = workspace.marginalLikelihood(for: entry)
      model = value
      shown = nil
      await value.load(history: workspace.history.entries)
    }
  }

  @ViewBuilder private func content(_ model: MarginalLikelihoodModel) -> some View {
    @Bindable var model = model
    if !model.available {
      Text(
        strings(
          "Marginal likelihoods are not offered for estimated FBD parameters: the sampling target then includes observation factors whose normalization has not been derived for model comparison."
        )
      )
      .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
    } else {
      HStack(alignment: .top, spacing: 18) {
        field("Powers", text: $model.powers, width: 60)
        field("Alpha", text: $model.alpha, width: 60)
        field("Steps per power", text: $model.steps, width: 110)
        field("Burn-in (%)", text: $model.burnin, width: 70)
        field("Seed", text: $model.seed, width: 130)
      }.disabled(model.busy)
      Text(
        strings(
          "Powers are quantiles of Beta(alpha, 1), from one down to zero; a smaller alpha places more of them near zero, where the mean log-likelihood changes fastest. Each power runs the steps above and logs every %@ steps, as the run did. Chains, checkpoints and automatic stopping are not available.",
          strings.number(model.sampleEvery))
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
      HStack(spacing: 12) {
        Button(strings("Estimate marginal likelihood")) { model.start() }
          .buttonStyle(LabButtonStyle(prominent: true)).disabled(model.busy || !model.valid)
        if let schedule = model.schedule, !model.busy {
          Text(strings("%@ steps in total", strings.number(schedule.powers * schedule.steps)))
            .font(.system(size: 11)).foregroundStyle(Lab.secondary)
        }
        if model.busy {
          ProgressView(value: fraction(model)).frame(width: 140)
          Text(
            strings(
              "Power %@ of %@ · step %@ of %@",
              strings.number(max(model.progress.powersStarted, 1)), model.powers,
              strings.number(model.progress.step), strings.number(Int(model.steps) ?? 0))
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary)
          Button(strings("Cancel")) { model.cancel() }.buttonStyle(LabButtonStyle(compact: true))
        }
      }
    }
    if let error = model.error {
      DiagnosticMessage(title: strings(error), details: model.details, strings: strings)
    }
    if !model.estimates.isEmpty {
      Divider().overlay(Lab.border)
      VStack(alignment: .leading, spacing: 10) {
        Eyebrow(text: strings("Saved estimates"))
        ForEach(model.estimates) { saved in
          DisclosureGroup(
            isExpanded: Binding(
              get: { shown == saved.id }, set: { shown = $0 ? saved.id : nil })
          ) {
            MarginalEstimateView(saved: saved, strings: strings).padding(.top, 12)
          } label: {
            VStack(alignment: .leading, spacing: 4) {
              Text(
                strings(
                  "Stepping stone %@ · Path sampling %@",
                  estimate(saved.estimate.steppingStone), estimate(saved.estimate.pathSampling))
              )
              .font(.system(size: 12, weight: .medium, design: .monospaced)).monospacedDigit()
              HStack(spacing: 10) {
                Text(schedule(saved.estimate.settings))
                Text(date(saved.createdAt)).foregroundStyle(Lab.muted)
              }.font(.system(size: 11)).foregroundStyle(Lab.secondary)
            }
          }
        }
      }
      if let newest = model.estimates.first, !model.others.isEmpty {
        Divider().overlay(Lab.border)
        comparison(newest, against: model.others)
      }
    }
  }

  private func comparison(_ own: SavedEstimate, against others: [SavedEstimate]) -> some View {
    VStack(alignment: .leading, spacing: 12) {
      Eyebrow(text: strings("Log Bayes factors"))
      Text(
        strings(
          "Estimates saved with other completed runs of the same matrix, constraints and gap policy, against this run's newest estimate. A difference is this model's estimate minus the other's: positive favours this model. Its standard error combines both estimates in quadrature."
        )
      )
      .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
      ScrollView(.horizontal) {
        Grid(alignment: .leading, horizontalSpacing: 22, verticalSpacing: 10) {
          GridRow {
            Text(strings("Run"))
            Text(strings("Model"))
            Text(strings("Stepping stone"))
            Text(strings("Difference"))
            Text(strings("Path sampling"))
            Text(strings("Difference"))
          }.font(.system(size: 10, weight: .semibold)).foregroundStyle(Lab.muted)
          Divider().gridCellColumns(6)
          ForEach(others) { other in
            let stone = MarginalLikelihoodModel.difference(
              own.estimate.steppingStone, other.estimate.steppingStone)
            let path = MarginalLikelihoodModel.difference(
              own.estimate.pathSampling, other.estimate.pathSampling)
            GridRow {
              Text(other.run.lastPathComponent).lineLimit(1).truncationMode(.middle)
                .frame(maxWidth: 180, alignment: .leading)
              Text(describe(other, against: own)).foregroundStyle(Lab.secondary)
              Text(estimate(other.estimate.steppingStone))
              Text(difference(stone))
              Text(estimate(other.estimate.pathSampling))
              Text(difference(path))
            }.font(.system(size: 11, design: .monospaced)).monospacedDigit()
          }
        }
      }.frame(maxWidth: .infinity, alignment: .leading)
      Text(
        strings(
          "Following Kass and Raftery (1995), a log Bayes factor above about 3 to 5 is usually read as strong support. Repeat the estimate with other seeds before reading a difference within a few standard errors. The marginal likelihood depends on the priors, so a Bayes factor compares models together with their priors."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
      if others.contains(where: { $0.record.model.conditioning != own.record.model.conditioning }) {
        Text(
          strings(
            "Mk, Mkv and Mkparsinf define likelihoods for different observation processes. A Bayes factor between them compares those processes as well as the models."
          )
        )
        .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
      }
    }
  }

  private func describe(_ saved: SavedEstimate, against own: SavedEstimate) -> String {
    let model = saved.record.model
    var text = strings(model.preset == "reference" ? "Relaxed clock" : "Strict clock")
    text += " · " + model.conditioningLabel
    if model.partitioning != own.record.model.partitioning {
      let observed = model.partitioning == "observed"
      text += " · " + strings(observed ? "By observed states" : "Declared alphabet")
    }
    return text
  }

  private func fraction(_ model: MarginalLikelihoodModel) -> Double {
    guard let schedule = model.schedule else { return 0 }
    let done = Double(max(model.progress.powersStarted - 1, 0)) * Double(schedule.steps)
    return min((done + Double(model.progress.step)) / Double(schedule.powers * schedule.steps), 1)
  }

  private func estimate(_ value: MarginalEstimate.Estimate) -> String {
    let text = number(value.logMarginalLikelihood, digits: 2)
    guard let error = value.standardError else { return text }
    return "\(text) ± \(number(error, digits: 2))"
  }

  private func difference(_ value: (value: Double, standardError: Double?)) -> String {
    let text = value.value.formatted(
      .number.precision(.fractionLength(2)).sign(strategy: .always()).locale(strings.locale))
    guard let error = value.standardError else { return text }
    return "\(text) ± \(number(error, digits: 2))"
  }

  private func schedule(_ value: MarginalEstimate.Schedule) -> String {
    strings(
      "%@ powers · alpha %@ · %@ steps per power · %@%% burn-in · seed %@",
      strings.number(value.powers), number(value.alpha, digits: 2),
      strings.number(value.stepsPerPower), String(value.burninPercent), String(value.seed))
  }

  private func number(_ value: Double, digits: Int) -> String {
    value.formatted(.number.precision(.fractionLength(digits)).locale(strings.locale))
  }

  private func date(_ value: Date?) -> String {
    value?.formatted(
      .dateTime.day().month(.abbreviated).year().hour().minute().locale(strings.locale))
      ?? strings("Date unavailable")
  }

  private func field(_ label: String, text: Binding<String>, width: CGFloat) -> some View {
    VStack(alignment: .leading, spacing: 6) {
      Text(strings(label)).font(.system(size: 11, weight: .medium))
      TextField(strings(label), text: text).textFieldStyle(.roundedBorder)
        .font(.system(size: 12, design: .monospaced)).frame(width: width).labelsHidden()
    }
  }
}

/// The per-power table of one saved estimate.
struct MarginalEstimateView: View {
  let saved: SavedEstimate
  let strings: AppStrings

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      ScrollView(.horizontal) {
        Grid(alignment: .leading, horizontalSpacing: 22, verticalSpacing: 8) {
          GridRow {
            Text(strings("Power"))
            Text(strings("Samples"))
            Text(strings("Mean log-likelihood"))
            Text(strings("SD"))
            Text("ESS")
          }.font(.system(size: 10, weight: .semibold)).foregroundStyle(Lab.muted)
          Divider().gridCellColumns(5)
          ForEach(saved.estimate.powers) { power in
            GridRow {
              Text(number(power.power, digits: 4)).foregroundStyle(Lab.secondary)
              Text(strings.number(power.samples))
              Text(number(power.meanLogLikelihood, digits: 2))
              Text(number(power.sdLogLikelihood, digits: 2))
              Text(
                (power.ess.map { number($0, digits: 0) } ?? "—")
                  + (power.lagLimitReached ? " *" : ""))
            }.font(.system(size: 11, design: .monospaced)).monospacedDigit()
          }
        }
      }.frame(maxWidth: .infinity, alignment: .leading)
      Text(
        strings(
          "* means the ESS autocorrelation estimate reached its lag limit. Low ESS near power zero is common: those chains explore the prior. Path sampling omits the error of the trapezoidal rule, so the two estimates can differ by more than their standard errors when the schedule is coarse."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
      HStack(spacing: 10) {
        Button(strings("Estimate record")) {
          NSWorkspace.shared.activateFileViewerSelecting([
            saved.directory.appendingPathComponent("marginal.json")
          ])
        }
        Button(strings("Run record")) {
          NSWorkspace.shared.activateFileViewerSelecting([
            saved.directory.appendingPathComponent("run.json")
          ])
        }
        Button(strings("Open folder")) { NSWorkspace.shared.open(saved.directory) }
      }.buttonStyle(LabButtonStyle(compact: true))
    }
  }

  private func number(_ value: Double, digits: Int) -> String {
    value.formatted(.number.precision(.fractionLength(digits)).locale(strings.locale))
  }
}
