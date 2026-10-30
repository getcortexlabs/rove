import SwiftUI

struct ProcessSettingsView: View {
  @Bindable var workspace: Workspace
  private var strings: AppStrings { workspace.strings }
  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      Text(strings("Fossilized birth–death process")).font(.system(size: 13, weight: .semibold))
      note(
        "Choose which values stay fixed and which Rove estimates. Each estimated parameter needs a starting value and a prior."
      )
      rate(
        "Birth rate · λ", value: $workspace.settings.dating.birthRate,
        prior: $workspace.settings.dating.estimation.birth)
      rate(
        "Death rate · μ", value: $workspace.settings.dating.deathRate,
        prior: $workspace.settings.dating.estimation.death)
      rate(
        "Fossil sampling · ψ", value: $workspace.settings.dating.fossilSamplingRate,
        prior: $workspace.settings.dating.estimation.fossilSampling)
      note(
        "Rates are per lineage, per time unit. A fixed death rate can be zero; estimated rates must start positive."
      )
      VStack(alignment: .leading, spacing: 10) {
        mode(
          "Present-day sampling · ρ",
          estimated: $workspace.settings.dating.estimation.estimateSampling)
        HStack(alignment: .top, spacing: 14) {
          field(
            workspace.settings.dating.estimation.estimateSampling
              ? "Initial probability" : "Fixed probability",
            text: $workspace.settings.dating.presentSampling)
          if workspace.settings.dating.estimation.estimateSampling {
            field("Beta prior · α", text: $workspace.settings.dating.estimation.samplingAlpha)
            field("Beta prior · β", text: $workspace.settings.dating.estimation.samplingBeta)
          }
        }
        note(
          "Sampling is the fraction of living lineages observed. A beta prior describes uncertainty between zero and one; its mean is α / (α + β)."
        )
      }
      Divider().overlay(Lab.border)
      rate(
        "Origin age", value: $workspace.settings.dating.origin,
        prior: $workspace.settings.dating.estimation.origin, origin: true)
      if workspace.settings.dating.estimation.origin.estimated {
        note(
          "The origin prior is exponential on the extra time before the oldest allowed tip age. Its mean is that oldest bound plus the mean excess. The origin must also stay older than the root."
        )
        if let oldest = workspace.settings.dating.oldestAge {
          Text(
            strings(
              "Oldest allowed tip age: %@ %@", TreeFigure.number(oldest),
              workspace.settings.dating.unit)
          )
          .font(.system(size: 11, weight: .medium)).foregroundStyle(Lab.ice)
        }
      } else {
        note(
          "The origin must be older than the root and every allowed tip age, including interval maxima."
        )
      }
      if workspace.settings.dating.estimatesProcess {
        VStack(alignment: .leading, spacing: 8) {
          Label(strings("How the fossils inform the process"), systemImage: "leaf")
            .font(.system(size: 12, weight: .medium)).foregroundStyle(Lab.ice)
          note(
            "The observed sample count, fossil age evidence and imposed terminal/clade constraints inform the estimated rates and origin. A run without character data still uses that information."
          )
        }.padding(14).background(Lab.ice.opacity(0.045), in: RoundedRectangle(cornerRadius: 12))
      }
    }
  }
  private func rate(
    _ title: String, value: Binding<String>, prior: Binding<ProcessRateInput>, origin: Bool = false
  ) -> some View {
    VStack(alignment: .leading, spacing: 10) {
      mode(title, estimated: prior.estimated)
      HStack(alignment: .top, spacing: 14) {
        field(prior.wrappedValue.estimated ? "Initial value" : "Fixed value", text: value)
        if prior.wrappedValue.estimated {
          field(origin ? "Mean origin excess" : "Exponential prior mean", text: prior.mean)
        }
      }
    }
  }
  private func mode(_ title: String, estimated: Binding<Bool>) -> some View {
    HStack {
      Text(strings(title)).font(.system(size: 12, weight: .medium))
      Spacer()
      Picker(strings(title), selection: estimated) {
        Text(strings("Keep fixed")).tag(false)
        Text(strings("Estimate")).tag(true)
      }.labelsHidden().pickerStyle(.segmented).frame(width: 210)
    }
  }
  private func field(_ label: String, text: Binding<String>) -> some View {
    VStack(alignment: .leading, spacing: 7) {
      Text(strings(label)).font(.system(size: 10)).foregroundStyle(Lab.secondary)
      TextField(strings("Required"), text: text).textFieldStyle(.roundedBorder)
        .font(.system(size: 12, design: .monospaced)).accessibilityLabel(strings(label))
    }.frame(maxWidth: .infinity, alignment: .leading)
  }
  private func note(_ key: String) -> some View {
    Text(strings(key)).font(.system(size: 11)).foregroundStyle(Lab.secondary)
      .lineSpacing(3).fixedSize(horizontal: false, vertical: true)
  }
}
