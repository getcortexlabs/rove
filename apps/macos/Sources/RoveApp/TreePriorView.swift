import SwiftUI

extension AnalysisSettings {
  var treePriorLabel: String {
    dating.enabled
      ? "Fossilized birth-death" : treePrior == "yule" ? "Yule · pure birth" : "Birth-death"
  }

  func treePriorDescription(strings: AppStrings) -> String {
    guard treePrior == "yule" else { return strings(treePriorLabel) }
    return strings(
      "Yule · birth rate Uniform(0, %@)", yuleBirthRateUpper.map(String.init(describing:)) ?? "—")
  }
}

struct TreePriorSection: View {
  @Bindable var workspace: Workspace
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 14) {
        Text(strings("Tree prior")).font(.system(size: 13, weight: .semibold))
        Picker(strings("Tree prior"), selection: $workspace.settings.treePrior) {
          Text(
            strings(workspace.settings.dating.enabled ? "Fossilized birth-death" : "Birth-death")
          )
          .tag("birth-death")
          Text(strings("Yule · pure birth")).tag("yule")
        }.labelsHidden().frame(maxWidth: 350)
        Text(
          strings(
            workspace.settings.treePrior == "yule"
              ? "Yule models speciation without extinction for contemporaneous species tips, with complete sampling. It does not support fossil ages, calibrations or population coalescence."
              : "Birth-death allows extinction as well as speciation. Fossil dating uses the fossilized birth-death model below."
          )
        )
        .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(3)
        .fixedSize(horizontal: false, vertical: true)
        if workspace.settings.treePrior == "yule" {
          HStack {
            Text(strings("Birth-rate upper bound")).font(.system(size: 12))
            TextField(strings("Birth-rate upper bound"), text: $workspace.settings.birthRateUpper)
              .textFieldStyle(.roundedBorder).frame(width: 110)
          }
          Text(
            strings(
              "The birth rate has a Uniform(0, upper bound) prior. Its units are inverse tree-height units on the fixed clock scale. Choose a defensible bound and inspect prior-only runs; 1000 is a starting value, not a general recommendation."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
          .fixedSize(horizontal: false, vertical: true)
          if workspace.settings.dating.enabled {
            Text(
              strings("Yule requires contemporaneous tips without fossil dating or calibrations.")
            )
            .font(.system(size: 12)).foregroundStyle(Lab.coral).fixedSize(
              horizontal: false, vertical: true)
          } else if workspace.settings.yuleBirthRateUpper == nil {
            Text(strings("Use a positive, finite upper bound for the Yule birth-rate prior."))
              .font(.system(size: 12)).foregroundStyle(Lab.coral).fixedSize(
                horizontal: false, vertical: true)
          }
        }
      }
    }
  }
}
