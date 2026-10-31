import SwiftUI

struct DatingSection: View {
  @Bindable var workspace: Workspace
  @State private var editingAges = false
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 18) {
        HStack(spacing: 14) {
          Image(systemName: "clock.arrow.circlepath").font(.system(size: 24, weight: .light))
            .foregroundStyle(Lab.ice)
          VStack(alignment: .leading, spacing: 5) {
            Text(strings("A place in time")).font(Lab.display(21, weight: .semibold))
            Text(strings("Use fossil ages to date the tree."))
              .font(.system(size: 12)).foregroundStyle(Lab.secondary)
          }
          Spacer()
          Toggle(
            strings("Date fossils"),
            isOn: Binding(
              get: { workspace.settings.dating.enabled }, set: { workspace.setDatingEnabled($0) }
            )
          ).toggleStyle(.switch).font(.system(size: 12))
        }
        if workspace.settings.dating.enabled {
          Divider().overlay(Lab.border)
          HStack(alignment: .top, spacing: 24) {
            field("Time unit", text: $workspace.settings.dating.timeUnit, placeholder: "Ma")
              .frame(width: 110)
            VStack(alignment: .leading, spacing: 8) {
              Text(strings("Ages before the present")).font(.system(size: 12, weight: .medium))
              Text(
                strings(
                  "Use one unit throughout. Ma means millions of years; present-day terminals have age zero."
                )
              )
              .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3).fixedSize(
                horizontal: false, vertical: true)
            }.frame(maxWidth: .infinity, alignment: .leading)
            Button {
              editingAges = true
            } label: {
              Label(strings("Edit tip ages"), systemImage: "slider.horizontal.3")
            }.buttonStyle(LabButtonStyle(compact: true))
          }
          HStack(spacing: 16) {
            Label(
              strings("%@ fossils", strings.number(workspace.settings.dating.fossilCount)),
              systemImage: "leaf")
            Text(
              strings(
                "%@ with age intervals", strings.number(workspace.settings.dating.intervalCount)))
          }.font(.system(size: 11)).foregroundStyle(Lab.ice)
          Text(
            strings(
              "Every terminal starts as present-day. Edit the fossil ages before running the analysis."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary)
          Divider().overlay(Lab.border)
          ProcessSettingsView(workspace: workspace)
          Divider().overlay(Lab.border)
          VStack(alignment: .leading, spacing: 12) {
            HStack {
              Text(strings("Global clock rate")).font(.system(size: 13, weight: .semibold))
              Spacer()
              Picker(strings("Global clock rate"), selection: $workspace.settings.dating.clock) {
                Text(strings("Estimate")).tag(DatingSettings.Clock.estimated)
                Text(strings("Keep fixed")).tag(DatingSettings.Clock.fixed)
              }.pickerStyle(.segmented).labelsHidden().frame(width: 230)
            }
            HStack(alignment: .top, spacing: 16) {
              field(
                workspace.settings.dating.clock == .estimated
                  ? "Initial clock rate" : "Fixed clock rate",
                text: $workspace.settings.dating.clockRate)
              if workspace.settings.dating.clock == .estimated {
                field("Exponential prior mean", text: $workspace.settings.dating.clockPriorMean)
              }
            }
            Text(
              strings(
                "Expected changes per character, per time unit. The evolutionary clock above controls whether branches share this pace or vary around it."
              )
            )
            .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3).fixedSize(
              horizontal: false, vertical: true)
          }
          Text(
            strings(
              "Fossils are constrained to terminal tips. Sampled ancestors and species occurrence ranges are not supported."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.muted).lineSpacing(3).fixedSize(
            horizontal: false, vertical: true)
          if let issue = workspace.settings.dating.validationError(taxa: workspace.report?.allTaxa)
          {
            InlineMessage(text: strings(issue), isError: true)
          }
        }
      }
    }.sheet(isPresented: $editingAges) { TipAgeEditor(workspace: workspace) }
  }

  private func field(_ title: String, text: Binding<String>, placeholder: String = "Required")
    -> some View
  {
    VStack(alignment: .leading, spacing: 8) {
      Text(strings(title)).font(.system(size: 11, weight: .medium)).foregroundStyle(Lab.secondary)
      TextField(strings(placeholder), text: text).textFieldStyle(.roundedBorder)
        .font(.system(size: 12, design: .monospaced)).accessibilityLabel(strings(title))
    }.frame(maxWidth: .infinity, alignment: .leading)
  }
}

struct TipAgeEditor: View {
  @Bindable var workspace: Workspace
  @Environment(\.dismiss) private var dismiss
  @State private var ages: [String: TipAgeInput]
  @State private var search = ""
  private var strings: AppStrings { workspace.strings }
  private var taxa: [String] { workspace.report?.allTaxa ?? [] }
  private var filtered: [String] {
    taxa.filter { search.isEmpty || $0.localizedCaseInsensitiveContains(search) }
  }
  private var issue: String? { DatingSettings.ageError(ages, taxa: taxa) }

  init(workspace: Workspace) {
    self.workspace = workspace
    _ages = State(initialValue: workspace.settings.dating.ages)
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 18) {
      HStack(alignment: .top) {
        VStack(alignment: .leading, spacing: 8) {
          Eyebrow(text: strings("Ages before the present"), color: Lab.ice)
          Text(strings("Give each fossil its time.")).font(Lab.display(28))
        }
        Spacer()
        Text(workspace.settings.dating.unit).font(
          .system(size: 13, weight: .semibold, design: .monospaced)
        )
        .foregroundStyle(Lab.ice).padding(10).background(
          Lab.ice.opacity(0.08), in: RoundedRectangle(cornerRadius: 10))
      }
      Text(
        strings(
          "Present-day means zero. Enter a fixed fossil age or a younger–older interval; interval ages are estimated with a uniform prior within those bounds."
        )
      )
      .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(3).fixedSize(
        horizontal: false, vertical: true)
      TextField(strings("Find a terminal"), text: $search).textFieldStyle(.roundedBorder)
      HStack {
        Text(strings("%@ terminals", strings.number(taxa.count)))
        Spacer()
        Text(
          strings("%@ fossils", strings.number(ages.values.filter { $0.kind != .present }.count)))
      }.font(.system(size: 11)).foregroundStyle(Lab.secondary)
      ScrollView {
        LazyVStack(spacing: 1) {
          ForEach(filtered, id: \.self) { taxon in
            TipAgeRow(
              taxon: taxon,
              age: Binding(
                get: { ages[taxon] ?? TipAgeInput() }, set: { ages[taxon] = $0 }
              ), strings: strings)
          }
          if filtered.isEmpty {
            Text(strings("No matching terminals.")).font(.system(size: 12)).foregroundStyle(
              Lab.secondary
            ).padding(24)
          }
        }
      }.frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Lab.surface, in: RoundedRectangle(cornerRadius: 12))
      if let issue {
        Text(strings(issue)).font(.system(size: 11)).foregroundStyle(Lab.coral).fixedSize(
          horizontal: false, vertical: true)
      }
      HStack {
        Button(strings("Cancel")) { dismiss() }.buttonStyle(LabButtonStyle(compact: true))
          .keyboardShortcut(.cancelAction)
        Spacer()
        Button(strings("Apply ages")) {
          guard issue == nil else { return }
          workspace.settings.dating.ages = ages
          dismiss()
        }.buttonStyle(LabButtonStyle(prominent: true, compact: true)).keyboardShortcut(
          .defaultAction
        ).disabled(issue != nil)
      }
    }.padding(28).frame(width: 730, height: 620).foregroundStyle(Lab.ink).background(Lab.background)
      .preferredColorScheme(.dark).tint(Lab.accent)
  }
}

private struct TipAgeRow: View {
  let taxon: String
  @Binding var age: TipAgeInput
  let strings: AppStrings
  var body: some View {
    HStack(spacing: 14) {
      Text(taxon).font(.system(size: 12, weight: .medium)).lineLimit(2).help(taxon)
        .frame(maxWidth: .infinity, alignment: .leading)
      Picker(strings("Age of %@", taxon), selection: $age.kind) {
        ForEach(TipAgeInput.Kind.allCases, id: \.self) { kind in Text(strings(kind.title)).tag(kind)
        }
      }.labelsHidden().frame(width: 150)
      Group {
        switch age.kind {
        case .present:
          Text("0").foregroundStyle(Lab.muted).frame(maxWidth: .infinity, alignment: .trailing)
        case .fixed:
          input("Age", value: $age.fixed)
        case .interval:
          HStack(spacing: 6) {
            input("Younger", value: $age.minimum)
            Text("–").foregroundStyle(Lab.muted)
            input("Older", value: $age.maximum)
          }
        }
      }.frame(width: 210)
    }.padding(14).background(age.kind == .present ? Lab.surface : Lab.ice.opacity(0.035))
  }
  private func input(_ label: String, value: Binding<String>) -> some View {
    TextField(strings(label), text: value).textFieldStyle(.roundedBorder)
      .font(.system(size: 12, design: .monospaced))
      .accessibilityLabel("\(taxon): \(strings(label))")
  }
}

struct DatingSummary: View {
  let dating: DatingSettings
  let strings: AppStrings
  var body: some View {
    if dating.enabled {
      VStack(alignment: .leading, spacing: 13) {
        Label(strings("Fossil tip dating"), systemImage: "clock.arrow.circlepath")
          .font(.system(size: 13, weight: .semibold)).foregroundStyle(Lab.ice)
        row("Time unit", dating.unit)
        row("Fossils", strings.number(dating.fossilCount))
        processRow("Birth rate · λ", value: dating.birthRate, prior: dating.estimation.birth)
        processRow("Death rate · μ", value: dating.deathRate, prior: dating.estimation.death)
        processRow(
          "Fossil sampling · ψ", value: dating.fossilSamplingRate,
          prior: dating.estimation.fossilSampling)
        row(
          "Present-day sampling · ρ",
          dating.presentSampling + " · "
            + strings(dating.estimation.estimateSampling ? "Initial value" : "Fixed"))
        if dating.estimation.estimateSampling {
          row(
            "Beta prior · α, β",
            "\(dating.estimation.samplingAlpha), \(dating.estimation.samplingBeta)")
        }
        processRow(
          "Origin age", value: "\(dating.origin) \(dating.unit)", prior: dating.estimation.origin,
          origin: true)
        row(dating.clock == .fixed ? "Fixed clock rate" : "Initial clock rate", dating.clockRate)
        if dating.clock == .estimated { row("Exponential prior mean", dating.clockPriorMean) }
        Text(
          strings(
            dating.estimatesProcess
              ? "Sample counts, age evidence and imposed terminal/clade constraints inform the estimated process parameters."
              : "FBD rates and origin are fixed. Ages in intervals and an estimated clock rate are sampled during the run."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3).fixedSize(
          horizontal: false, vertical: true)
        DisclosureGroup(strings("Tip ages · %@", dating.unit)) {
          ScrollView {
            LazyVStack(spacing: 10) {
              ForEach(dating.ages.keys.sorted(), id: \.self) { taxon in
                if let input = dating.ages[taxon] {
                  HStack {
                    Text(taxon).textSelection(.enabled)
                    Spacer()
                    Text(ageLabel(input)).foregroundStyle(Lab.secondary)
                  }.font(.system(size: 11))
                }
              }
            }.padding(.top, 10)
          }.frame(height: min(230, CGFloat(dating.ages.count * 25 + 10)))
        }.font(.system(size: 12))
      }
    }
  }
  private func processRow(
    _ title: String, value: String, prior: ProcessRateInput, origin: Bool = false
  ) -> some View {
    VStack(alignment: .leading, spacing: 8) {
      row(title, value + " · " + strings(prior.estimated ? "Initial value" : "Fixed"))
      if prior.estimated {
        row(origin ? "Mean origin excess" : "Exponential prior mean", prior.mean)
        if origin, let oldest = dating.oldestAge {
          row("Origin prior offset", "\(TreeFigure.number(oldest)) \(dating.unit)")
        }
      }
    }
  }
  private func row(_ title: String, _ value: String) -> some View {
    HStack(alignment: .top) {
      Text(strings(title)).foregroundStyle(Lab.secondary)
      Spacer(minLength: 16)
      Text(value).textSelection(.enabled)
    }.font(.system(size: 12))
  }
  private func ageLabel(_ input: TipAgeInput) -> String {
    switch input.kind {
    case .present: strings("Present-day") + " · 0"
    case .fixed: "\(input.fixed) \(dating.unit)"
    case .interval: "\(input.minimum)–\(input.maximum) \(dating.unit)"
    }
  }
}
