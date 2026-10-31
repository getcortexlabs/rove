import SwiftUI

/// Engine options beyond the exploration plan, collapsed by default.
struct AdvancedOptionsSection: View {
  @Bindable var workspace: Workspace
  @State private var expanded = false
  private var strings: AppStrings { workspace.strings }
  private var settings: AnalysisSettings { workspace.settings }

  var body: some View {
    LabPanel {
      DisclosureGroup(isExpanded: $expanded) {
        VStack(alignment: .leading, spacing: 22) {
          prior
          if settings.chains > 1 && settings.stopWhenConverged { thresholds }
          checkpoints
          schedule
        }.padding(.top, 18)
      } label: {
        HStack {
          VStack(alignment: .leading, spacing: 6) {
            Text(strings("Advanced sampling")).font(.system(size: 13, weight: .semibold))
            Text(strings("Engine options for people who read the sampling reports."))
              .font(.system(size: 11)).foregroundStyle(Lab.secondary)
          }
          Spacer()
          if settings.usesAdvancedOptions {
            Text(strings("In use")).font(.system(size: 10, weight: .medium))
              .foregroundStyle(Lab.ice).padding(.horizontal, 9).padding(.vertical, 5)
              .background(.white.opacity(0.045), in: Capsule())
          }
        }
      }.font(.system(size: 12))
    }
    .onAppear { if settings.usesAdvancedOptions { expanded = true } }
  }

  private var prior: some View {
    VStack(alignment: .leading, spacing: 8) {
      Toggle(strings("Sample from the priors only"), isOn: $workspace.settings.sampleFromPrior)
        .font(.system(size: 12))
      Text(
        strings(
          "The characters are ignored. The run shows what the tree and parameter priors imply before any data, which is useful before choosing them."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
    }
  }

  private var thresholds: some View {
    VStack(alignment: .leading, spacing: 10) {
      Eyebrow(text: strings("Stopping thresholds"))
      HStack(alignment: .top, spacing: 18) {
        numberField(
          "Minimum ESS", value: $workspace.settings.minESS, placeholder: "200",
          format: decimalFormat)
        numberField(
          "Maximum R-hat", value: $workspace.settings.maxRhat, placeholder: "1.01",
          format: decimalFormat)
        numberField(
          "Check every (steps)", value: $workspace.settings.checkEvery,
          placeholder: strings("Engine default"), format: .number.grouping(.never))
      }
      Text(
        strings(
          "Every traced statistic must reach these in every chain at two consecutive checks. Empty fields keep the engine's defaults."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
    }
  }

  private var checkpoints: some View {
    VStack(alignment: .leading, spacing: 10) {
      Eyebrow(text: strings("Checkpoints"))
      HStack(alignment: .top, spacing: 18) {
        numberField(
          "Save a checkpoint every (steps)", value: $workspace.settings.checkpointEvery,
          placeholder: strings("Every 30 seconds"), format: .number.grouping(.never))
        Text(
          strings(
            "A multiple of the sampling interval. By default the engine saves about every 30 seconds."
          )
        )
        .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
      }
    }
  }

  private var presetSelection: Binding<String> {
    Binding(
      get: { presetID },
      set: { id in
        if id == "classical" {
          workspace.settings.weights = []
        } else if let preset = ProposalSchedule.presets.first(where: { $0.id == id }) {
          workspace.settings.weights = preset.weights
        }
      })
  }

  private var presetID: String {
    if settings.weights.isEmpty { return "classical" }
    return ProposalSchedule.presets.first { $0.weights == settings.weights }?.id ?? "custom"
  }

  private var schedule: some View {
    VStack(alignment: .leading, spacing: 12) {
      Eyebrow(text: strings("Proposal schedule"))
      Picker(strings("Proposal schedule"), selection: presetSelection) {
        Text(strings("Classical · default")).tag("classical")
        ForEach(ProposalSchedule.presets) { preset in
          Text("\(strings(preset.name)) · \(preset.report)").tag(preset.id)
        }
        if presetID == "custom" { Text(strings("Custom")).tag("custom") }
      }.labelsHidden().frame(width: 400)
      Text(
        strings(
          "The evaluated schedules passed the posterior checks in their reports but lowered the efficiency of other parameters, so neither is the default. Weights replace the classical value of the named move."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
      OverrideEditor(
        title: strings("Selection weights"), addLabel: strings("Add weight"),
        overrides: $workspace.settings.weights, moves: ProposalSchedule.moves, strings: strings)
      Toggle(strings("Tune step sizes during the run"), isOn: $workspace.settings.tuning)
        .font(.system(size: 12))
      OverrideEditor(
        title: strings("Starting step sizes"), addLabel: strings("Add step size"),
        overrides: $workspace.settings.tuningValues,
        moves: ProposalSchedule.moves.filter { ProposalSchedule.tunable.contains($0) },
        strings: strings)
      Text(
        strings(
          "Step sizes are slide sizes for subtree_slide and scale factors below 1 otherwise. With tuning off they stay fixed, as in the BEAST comparisons."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
    }
  }

  private func numberField<V, F: ParseableFormatStyle>(
    _ label: String, value: Binding<V?>, placeholder: String, format: F
  ) -> some View where F.FormatInput == V, F.FormatOutput == String {
    VStack(alignment: .leading, spacing: 6) {
      Text(strings(label)).font(.system(size: 11, weight: .medium))
      TextField(strings(label), value: value, format: format, prompt: Text(placeholder))
        .textFieldStyle(.roundedBorder).font(.system(size: 12, design: .monospaced))
        .frame(width: 150).labelsHidden()
    }
  }
}

/// Enough digits to show any committed value unchanged.
private let decimalFormat = FloatingPointFormatStyle<Double>.number.grouping(.never)
  .precision(.significantDigits(1...12))

/// Editable move and value pairs. Each row's picker offers only moves no
/// other row uses, so rows keep distinct identities.
private struct OverrideEditor: View {
  let title: String
  let addLabel: String
  @Binding var overrides: [MoveOverride]
  let moves: [String]
  let strings: AppStrings

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text(title).font(.system(size: 11, weight: .medium))
      ForEach($overrides) { $override in
        HStack(spacing: 10) {
          Picker(strings("Move"), selection: $override.move) {
            ForEach(available(for: override.move), id: \.self) { Text($0).tag($0) }
          }.labelsHidden().frame(width: 210)
          TextField(strings("Value"), value: $override.value, format: decimalFormat)
            .textFieldStyle(.roundedBorder).font(.system(size: 12, design: .monospaced))
            .frame(width: 110).labelsHidden()
          Button {
            overrides.removeAll { $0.move == override.move }
          } label: {
            Image(systemName: "minus.circle")
          }.buttonStyle(.borderless).accessibilityLabel(strings("Remove"))
        }
      }
      if let next = available(for: nil).first {
        Button(addLabel) { overrides.append(MoveOverride(move: next, value: 1)) }
          .buttonStyle(.borderless).font(.system(size: 11))
      }
    }
  }

  private func available(for current: String?) -> [String] {
    moves.filter { move in move == current || !overrides.contains { $0.move == move } }
  }
}

extension AnalysisSettings {
  /// Review and History rows for options beyond the exploration plan.
  func advancedRows(strings: AppStrings) -> [(String, String)] {
    var rows: [(String, String)] = []
    if sampleFromPrior {
      rows.append(("Sampling target", strings("Priors only · characters ignored")))
    }
    if stopWhenConverged {
      var thresholds = strings(
        "ESS ≥ %@ · R-hat ≤ %@", minESS.map(Self.text) ?? "200", maxRhat.map(Self.text) ?? "1.01")
      if let checkEvery {
        thresholds += " · " + strings("check every %@ steps", strings.number(checkEvery))
      }
      rows.append(("Stopping thresholds", thresholds))
    }
    if let checkpointEvery {
      rows.append(("Checkpoints", strings("Every %@ steps", strings.number(checkpointEvery))))
    }
    if !tuning { rows.append(("Step-size tuning", strings("Off"))) }
    if !weights.isEmpty { rows.append(("Selection weights", Self.list(weights))) }
    if !tuningValues.isEmpty { rows.append(("Starting step sizes", Self.list(tuningValues))) }
    return rows
  }

  private static func list(_ overrides: [MoveOverride]) -> String {
    overrides.map { "\($0.move) = \(text($0.value))" }.joined(separator: ", ")
  }

  static func text(_ value: Double) -> String {
    value.formatted(
      .number.grouping(.never).precision(.significantDigits(1...12))
        .locale(Locale(identifier: "en_US_POSIX")))
  }
}
