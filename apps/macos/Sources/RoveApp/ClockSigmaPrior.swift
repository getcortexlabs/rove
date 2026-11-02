import SwiftUI

struct ClockSigmaPrior: Codable, Equatable, Sendable {
  let shape: Double
  let scale: Double
  static let reference = Self(shape: 5, scale: 0.2)

  func description(strings: AppStrings) -> String {
    strings("Gamma · shape %@ · scale %@", String(shape), String(scale))
  }
}

struct ClockSigmaPriorSection: View {
  @Bindable var workspace: Workspace
  @State private var expanded = false
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    LabPanel {
      DisclosureGroup(isExpanded: $expanded) {
        VStack(alignment: .leading, spacing: 14) {
          Text(
            strings(
              "Sigma controls how much branch rates can differ. This Gamma distribution expresses what you expect about sigma before looking at the characters."
            )
          )
          .font(.system(size: 12)).foregroundStyle(Lab.secondary)
          HStack(spacing: 18) {
            field("Shape · Alpha", text: $workspace.settings.clockSigmaShape)
            field("Scale · Beta", text: $workspace.settings.clockSigmaScale)
            Button(strings("Restore defaults")) {
              workspace.settings.clockSigmaShape = "5"
              workspace.settings.clockSigmaScale = "0.2"
            }.buttonStyle(.borderless).tint(Lab.accent)
          }
          Text(
            strings(
              "Beta is a scale, not a rate. The expected sigma is Alpha × Beta (1 with the defaults). These settings do not change gamma variation across characters and are not a shortcut to convergence."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
          if workspace.settings.clockSigmaPrior == nil {
            Text(strings("Use positive, finite shape and scale values for the clock sigma prior."))
              .font(.system(size: 12)).foregroundStyle(Lab.accent)
          }
        }.padding(.top, 14)
      } label: {
        HStack {
          Text(strings("Clock sigma prior")).font(.system(size: 13, weight: .semibold))
          Spacer()
          if let prior = workspace.settings.clockSigmaPrior {
            Text(prior.description(strings: strings)).font(.system(size: 11)).foregroundStyle(
              Lab.secondary)
          }
        }
      }.tint(Lab.accent)
        .onAppear { expanded = workspace.settings.usesCustomSigmaPrior }
    }
  }

  private func field(_ title: String, text: Binding<String>) -> some View {
    VStack(alignment: .leading, spacing: 6) {
      Text(strings(title)).font(.system(size: 11)).foregroundStyle(Lab.secondary)
      TextField(strings(title), text: text).textFieldStyle(.roundedBorder).frame(width: 140)
    }
  }
}
