import SwiftUI

struct SetupView: View {
  @Bindable var workspace: Workspace
  @State private var customPlan = false
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    VStack(alignment: .leading, spacing: 24) {
      VStack(alignment: .leading, spacing: 10) {
        Eyebrow(text: strings("02 / A few ground rules"), color: Lab.accent)
        Text(strings("How should we explore?")).font(Lab.display(35)).tracking(-1)
        Text(strings("Choose the assumptions behind your analysis."))
          .font(.system(size: 13)).foregroundStyle(Lab.secondary)
      }.padding(.top, 16)
      RootingSection(workspace: workspace)

      VStack(alignment: .leading, spacing: 12) {
        HStack {
          Text(strings("Evolutionary clock")).font(.system(size: 13, weight: .semibold))
          Spacer()
          help(
            "The clock links branch duration to expected character changes. Enable fossil dating below to supply ages and a time scale."
          )
        }
        HStack(spacing: 14) {
          clockCard(
            "strict", title: "One shared pace", subtitle: "Strict clock", icon: "metronome",
            description:
              "Every branch evolves at the same rate. One rate category across characters.")
          clockCard(
            "reference", title: "Room to wander", subtitle: "Relaxed clock + gamma",
            icon: "point.topleft.down.to.point.bottomright.curvepath",
            description:
              "Rates can vary across branches. Four gamma categories account for variation across characters."
          )
        }
      }

      if workspace.settings.clock == "reference" { ClockSigmaPriorSection(workspace: workspace) }

      DatingSection(workspace: workspace)

      LabPanel {
        VStack(alignment: .leading, spacing: 15) {
          HStack {
            VStack(alignment: .leading, spacing: 6) {
              Text(strings("Which characters did you include?")).font(
                .system(size: 13, weight: .semibold))
              Text(strings("Match the model to how your matrix was collected."))
                .font(.system(size: 11)).foregroundStyle(Lab.secondary)
            }
            Spacer()
            Picker(strings("Character selection"), selection: $workspace.settings.conditioning) {
              Text(strings("Variable only · Mkv")).tag("variable")
              Text(strings("Informative only · Mkparsinf")).tag("informative")
              Text(strings("All characters · Mk")).tag("none")
            }.labelsHidden().frame(width: 230).tint(Lab.accent)
          }
          Text(
            strings(
              workspace.settings.conditioning == "variable"
                ? "Use Mkv when constant characters were excluded. Missing observations can make a character's variability impossible to establish."
                : workspace.settings.conditioning == "informative"
                  ? "Use Mkparsinf only when constant and other parsimony-uninformative characters were excluded when the matrix was collected. Every retained character must remain informative despite missing or uncertain observations."
                  : "Use Mk when your sampling includes constant as well as variable characters. The matrix can contain both."
            )
          )
          .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
          if workspace.settings.conditioning == "variable",
            let issue = workspace.report?.variableError
          {
            DiagnosticMessage(
              title: strings(
                "Mkv requires characters known to be variable. Check the matrix or choose Mk if it matches how your data were collected."
              ), details: issue, strings: strings)
          }
          if workspace.settings.conditioning == "informative",
            let issue = workspace.report?.informativeError
          {
            DiagnosticMessage(
              title: strings(
                workspace.report?.partitions?.contains(where: { $0.ordering == .ordered }) == true
                  ? "Informative-only coding is not available for ordered characters. Keep the coding consistent with how your matrix was collected."
                  : "Informative-only coding requires at least two states each shared by two or more taxa. Check the matrix and how its characters were selected."
              ), details: issue, strings: strings)
          }
        }
      }

      LabPanel {
        VStack(alignment: .leading, spacing: 19) {
          Text(strings("The exploration plan")).font(.system(size: 13, weight: .semibold))
          HStack(spacing: 20) {
            VStack(alignment: .leading, spacing: 8) {
              Eyebrow(text: strings("Independent chains"))
              Picker(strings("Independent chains"), selection: $workspace.settings.chains) {
                ForEach(1...16, id: \.self) { count in
                  Text(strings(count == 1 ? "1 chain" : "%@ chains", String(count))).tag(count)
                }
              }.labelsHidden().frame(width: 200)
                .onChange(of: workspace.settings.chains) { _, count in
                  if count == 1 { workspace.settings.stopWhenConverged = false }
                }
            }
            Text(
              strings(
                "Separate starting trees and seeds, running at the same time. Steps below apply to each chain."
              )
            )
            .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
          }
          if workspace.settings.chains > 1 {
            HStack {
              Text(strings("Comparison burn-in (%)")).font(.system(size: 12))
              TextField(
                strings("Comparison burn-in (%)"),
                text: integerText($workspace.settings.comparisonBurnin)
              )
              .textFieldStyle(.roundedBorder).frame(width: 70)
            }
            Toggle(
              strings("Stop when diagnostic thresholds are met"),
              isOn: $workspace.settings.stopWhenConverged
            )
            .font(.system(size: 12))
            Text(
              strings(
                "Uses the comparison burn-in above and the advanced stopping thresholds (default ESS ≥ 200 and R-hat ≤ 1.01), with two consecutive checks and at least 1,000 retained samples per chain. Tree agreement still needs review."
              )
            )
            .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
          }
          Toggle(strings("Custom sampling plan"), isOn: $customPlan).font(.system(size: 12))
          HStack(alignment: .top, spacing: 25) {
            VStack(alignment: .leading, spacing: 9) {
              Eyebrow(text: strings("Chain length"))
              if customPlan {
                TextField(strings("Steps"), text: integerText($workspace.settings.steps))
                  .textFieldStyle(.roundedBorder)
              } else {
                Picker(
                  strings("Chain length"),
                  selection: Binding(
                    get: { workspace.settings.samplingPreset?.steps ?? 0 },
                    set: { value in
                      if let preset = SamplingPreset.all.first(where: { $0.steps == value }) {
                        workspace.settings.apply(preset)
                      }
                    })
                ) {
                  if workspace.settings.samplingPreset == nil {
                    Text(strings("%@ steps · custom", strings.number(workspace.settings.steps)))
                      .tag(
                        0)
                  }
                  ForEach(SamplingPreset.all) { preset in
                    Text(
                      preset.millions == 1
                        ? strings("1 million steps")
                        : strings("%@ million steps", strings.number(preset.millions))
                    ).tag(preset.steps)
                  }
                }.labelsHidden().frame(maxWidth: .infinity)
              }
              Text(strings("More steps allow more exploration.")).font(.system(size: 10))
                .foregroundStyle(
                  Lab.muted)
            }
            VStack(alignment: .leading, spacing: 9) {
              Eyebrow(text: strings("Save a sample every"))
              if customPlan {
                TextField(
                  strings("Sampling interval"), text: integerText($workspace.settings.sampleEvery)
                )
                .textFieldStyle(.roundedBorder)
              } else {
                Picker(strings("Sampling interval"), selection: $workspace.settings.sampleEvery) {
                  if !SamplingPreset.all.map(\.sampleEvery).contains(workspace.settings.sampleEvery)
                  {
                    Text(strings("%@ steps", strings.number(workspace.settings.sampleEvery))).tag(
                      workspace.settings.sampleEvery)
                  }
                  ForEach(SamplingPreset.all) { preset in
                    Text(strings("%@ steps", strings.number(preset.sampleEvery))).tag(
                      preset.sampleEvery)
                  }
                }.labelsHidden().frame(maxWidth: .infinity)
              }
              Text(strings("Controls how often a state is recorded.")).font(.system(size: 10))
                .foregroundStyle(Lab.muted)
            }
            VStack(alignment: .leading, spacing: 9) {
              Eyebrow(text: strings("Random seed"))
              TextField(strings("Seed"), text: $workspace.settings.seed)
                .textFieldStyle(.roundedBorder).font(.system(size: 12, design: .monospaced))
              Text(strings("A repeatable starting point.")).font(.system(size: 10)).foregroundStyle(
                Lab.muted)
            }.frame(width: 155)
          }
          Text(
            strings(
              "Presets save 10,000 samples per chain, plus the initial state, before burn-in. Changing the interval makes the plan custom. Saved samples are not independent samples (ESS)."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
          if !workspace.settings.samplingIntervalAdjustments.isEmpty {
            VStack(alignment: .leading, spacing: 10) {
              Text(
                strings(
                  "These intervals must be multiples of the sampling interval. Review the suggested values before applying them."
                )
              )
              .font(.system(size: 11)).foregroundStyle(Lab.secondary)
              ForEach(workspace.settings.samplingIntervalAdjustments, id: \.label) { adjustment in
                Text("\(strings(adjustment.label)): \(strings.number(adjustment.value))")
                  .font(.system(size: 11, design: .monospaced))
              }
              Button(strings("Apply interval adjustments")) {
                workspace.settings.alignSamplingIntervals()
              }
              .buttonStyle(LabButtonStyle(compact: true))
            }.padding(14).background(
              Lab.lilac.opacity(0.06), in: RoundedRectangle(cornerRadius: 12))
          }
          Text(
            strings(
              "Chain length alone doesn't establish convergence. Independent runs and sampling diagnostics are needed to assess an analysis."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
        }
      }
      .onAppear { customPlan = workspace.settings.samplingPreset == nil }
      AdvancedOptionsSection(workspace: workspace)
      GroupSection(workspace: workspace)
      if let error = workspace.settings.validationError {
        InlineMessage(text: strings(error), isError: true)
      }
    }
  }

  // Invalid or incomplete input clears the numeric value, so Review cannot
  // silently run the previous plan while a field displays something else.
  private func integerText(_ value: Binding<Int>) -> Binding<String> {
    Binding(
      get: { value.wrappedValue < 0 ? "" : String(value.wrappedValue) },
      set: { value.wrappedValue = Int($0) ?? -1 })
  }

  private func clockCard(
    _ value: String, title: String, subtitle: String, icon: String, description: String
  ) -> some View {
    let selected = workspace.settings.clock == value
    return Button {
      workspace.settings.clock = value
    } label: {
      VStack(alignment: .leading, spacing: 12) {
        HStack {
          Image(systemName: icon).font(.system(size: 22, weight: .light)).foregroundStyle(
            selected ? Lab.accent : Lab.lilac)
          Spacer()
          Image(systemName: selected ? "checkmark.circle.fill" : "circle")
            .foregroundStyle(selected ? Lab.accent : Lab.border)
        }
        VStack(alignment: .leading, spacing: 5) {
          Text(strings(title)).font(Lab.display(21, weight: .semibold)).foregroundStyle(Lab.ink)
          Text(strings(subtitle)).font(.system(size: 11, weight: .medium)).foregroundStyle(
            selected ? Lab.accent : Lab.secondary)
        }
        Text(strings(description)).font(.system(size: 12)).foregroundStyle(Lab.secondary)
          .lineSpacing(4).fixedSize(horizontal: false, vertical: true)
      }.frame(maxWidth: .infinity, minHeight: 155, alignment: .topLeading).padding(21)
        .background(
          selected ? Lab.accent.opacity(0.045) : Lab.surface, in: RoundedRectangle(cornerRadius: 16)
        )
        .overlay(
          RoundedRectangle(cornerRadius: 16).strokeBorder(
            selected ? Lab.accent.opacity(0.65) : Lab.border))
    }.buttonStyle(.plain).accessibilityLabel("\(strings(subtitle)). \(strings(description))")
      .accessibilityAddTraits(selected ? .isSelected : [])
  }

  private func help(_ message: String) -> some View {
    Image(systemName: "questionmark.circle").foregroundStyle(Lab.muted).help(strings(message))
      .accessibilityLabel(strings(message))
  }
}

struct ReviewView: View {
  @Bindable var workspace: Workspace
  private var strings: AppStrings { workspace.strings }
  let report: MatrixReport

  var body: some View {
    VStack(alignment: .leading, spacing: 24) {
      VStack(alignment: .leading, spacing: 10) {
        Eyebrow(text: strings("03 / Check your bearings"), color: Lab.accent)
        Text(strings("A good place to begin.")).font(Lab.display(35)).tracking(-1)
        Text(strings("Your matrix and assumptions, all in one place.")).font(.system(size: 13))
          .foregroundStyle(Lab.secondary)
      }.padding(.top, 16)

      LabPanel {
        VStack(spacing: 0) {
          HStack(spacing: 13) {
            Image(systemName: "square.grid.3x3.fill").font(.system(size: 25)).foregroundStyle(
              Lab.lilac)
            VStack(alignment: .leading, spacing: 5) {
              Text(workspace.file?.lastPathComponent ?? strings("Matrix")).font(
                .system(size: 14, weight: .semibold))
              Text(
                strings.matrixSummary(
                  taxa: report.taxonCount, characters: report.characterCount,
                  patterns: report.patternCount)
              )
              .font(.system(size: 11)).foregroundStyle(Lab.secondary)
            }
            Spacer()
            Text("NEXUS").font(.system(size: 10, design: .monospaced)).foregroundStyle(Lab.muted)
          }.padding(.bottom, 22)
          rule
          row(
            "Character model",
            workspace.settings.conditioning == "variable"
              ? "Mkv · variable characters"
              : workspace.settings.conditioning == "informative"
                ? "Mkparsinf · informative characters" : "Mk · all characters")
          if let partitions = report.partitions {
            row(
              "State partitions",
              partitions.map {
                strings.partitionSummary($0)
              }.joined(separator: " / "))
          }
          row(
            "Evolutionary clock",
            workspace.settings.clock == "reference"
              ? "Lognormal relaxed clock · 4 gamma categories" : "Strict clock · 1 rate category")
          if let prior = workspace.settings.clockSigmaPrior {
            row("Clock sigma prior", prior.description(strings: strings))
          }
          row(
            "Tree prior",
            workspace.settings.dating.estimatesProcess
              ? "Fossilized birth–death · estimated parameters"
              : workspace.settings.dating.enabled
                ? "Fossilized birth–death · fixed process" : "Birth–death · all tips at height zero"
          )
          rule
          row(
            "Chain",
            strings(
              "%@ steps · seed %@", strings.number(workspace.settings.steps),
              workspace.settings.seed))
          row("Independent chains", String(workspace.settings.chains))
          row("Sampling", strings("Every %@ steps", strings.number(workspace.settings.sampleEvery)))
          row("Gaps", workspace.settings.gapsAsMissing ? "Treat as missing" : "Reject")
          ForEach(workspace.settings.advancedRows(strings: strings), id: \.0) { label, value in
            row(label, value)
          }
        }
      }

      if workspace.settings.dating.enabled {
        LabPanel { DatingSummary(dating: workspace.settings.dating, strings: strings) }
      }
      LabPanel {
        ConstraintSummary(
          outgroup: workspace.outgroup, taxa: report.allTaxa,
          groups: workspace.constraints, strings: strings)
      }
      HStack(alignment: .top, spacing: 13) {
        Image(systemName: "leaf").font(.system(size: 23)).foregroundStyle(Lab.accent)
        VStack(alignment: .leading, spacing: 8) {
          Text(strings("Ready to keep exploring.")).font(Lab.display(20, weight: .semibold))
          Text(
            strings(
              "Start the analysis on this Mac. Your reviewed settings and input fingerprint will be saved with the output."
            )
          )
          .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
        }
      }.padding(21).frame(maxWidth: .infinity, alignment: .leading)
        .background(Lab.accent.opacity(0.045), in: RoundedRectangle(cornerRadius: 15))
      LabPanel {
        VStack(alignment: .leading, spacing: 12) {
          HStack {
            Eyebrow(text: strings("Save runs in"))
            Spacer()
            Button(strings("Choose folder…"), action: workspace.chooseOutputFolder).buttonStyle(
              LabButtonStyle(compact: true))
          }
          Text(workspace.outputParent.path).font(.system(size: 11, design: .monospaced))
            .foregroundStyle(Lab.secondary).textSelection(.enabled)
          Text(strings("Each run gets its own folder. Existing results are never overwritten."))
            .font(.system(size: 11)).foregroundStyle(Lab.muted)
        }
      }
      if let issue = workspace.issue { InlineMessage(text: strings(issue), isError: true) }
      if let notice = workspace.notice { InlineMessage(text: strings(notice)) }
      if !workspace.settings.sampleFromPrior && !workspace.settings.dating.estimatesProcess {
        Button(action: workspace.prepareModelComparison) {
          Label(strings("Estimate model evidence…"), systemImage: "scalemass")
        }.buttonStyle(LabButtonStyle(compact: true)).disabled(!workspace.canStartRun)
        Text(
          strings(
            "Open a separate power-posterior plan for this model. No earlier sampling run is required."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary)
      }
      HStack(spacing: 12) {
        Button(action: workspace.saveDraft) {
          Label(strings("Save setup"), systemImage: "square.and.arrow.down")
        }.buttonStyle(LabButtonStyle(compact: true))
        Button(action: workspace.copyCommand) {
          Label(strings("Copy terminal command"), systemImage: "terminal")
        }
        .buttonStyle(LabButtonStyle(compact: true))
        if let saved = workspace.savedURL {
          Button {
            NSWorkspace.shared.activateFileViewerSelecting([saved])
          } label: {
            Label(strings("Show saved setup"), systemImage: "folder")
          }.buttonStyle(LabButtonStyle(compact: true))
        }
        Spacer()
        Text(strings("A setup file references your matrix; it doesn't copy it."))
          .font(.system(size: 10)).foregroundStyle(Lab.muted).frame(maxWidth: 215)
      }
    }
  }

  private var rule: some View {
    Rectangle().fill(Lab.border).frame(height: 1).padding(.vertical, 6)
  }
  private func row(_ label: String, _ value: String) -> some View {
    HStack(alignment: .top) {
      Text(strings(label)).foregroundStyle(Lab.secondary)
      Spacer(minLength: 20)
      Text(strings(value)).foregroundStyle(Lab.ink).multilineTextAlignment(.trailing)
    }.font(.system(size: 12)).padding(.vertical, 11)
  }
}
