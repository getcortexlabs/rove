import SwiftUI

struct MixedSourcesSection: View {
  let metadata: MixedInputMetadata
  let strings: AppStrings
  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 12) {
        Text(strings("Morphology + DNA")).font(.system(size: 13, weight: .semibold))
        Text(
          strings(
            "Each block keeps its own alphabet. Morphological conditioning and alphabet choices apply only to morphology; DNA always uses all four bases and includes constant sites."
          )
        )
        .font(.system(size: 12)).foregroundStyle(Lab.secondary).fixedSize(
          horizontal: false, vertical: true)
        ForEach(Array(metadata.sources.enumerated()), id: \.offset) { _, source in
          VStack(alignment: .leading, spacing: 5) {
            Text(
              source.name + " · "
                + (source.dataType == "dna" ? "DNA · JC69" : strings("Morphology"))
            )
            .font(.system(size: 12, weight: .medium))
            Text("SHA-256 · " + source.sha256).font(.system(size: 9, design: .monospaced))
              .foregroundStyle(Lab.muted).textSelection(.enabled)
            if !source.missingTaxa.isEmpty {
              Text(
                strings(
                  "Absent taxa filled with missing cells: %@",
                  source.missingTaxa.joined(separator: ", "))
              )
              .font(.system(size: 11)).foregroundStyle(Lab.secondary).fixedSize(
                horizontal: false, vertical: true)
            }
          }
        }
        Text(
          strings(
            metadata.missingTaxaPolicy == "reject"
              ? "Require the same taxa" : "Keep the union; absent cells are missing")
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary)
      }
    }
  }
}

struct MixedRateSection: View {
  @Bindable var workspace: Workspace
  private var strings: AppStrings { workspace.strings }
  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 12) {
        Text(strings("Morphology/DNA rate ratio")).font(.system(size: 13, weight: .semibold))
        Text(
          strings(
            "Both blocks share the topology, branch clock and gamma shape. DNA fixes the reference rate at 1; the ratio scales every morphological partition. Separate clocks are not available."
          )
        )
        .font(.system(size: 12)).foregroundStyle(Lab.secondary).fixedSize(
          horizontal: false, vertical: true)
        Picker(
          strings("Morphology/DNA rate ratio"), selection: $workspace.settings.morphologyRateMode
        ) {
          Text(strings("Choose a rate model")).tag("")
          Text(strings("Estimate with a lognormal prior")).tag("estimate")
          Text(strings("Fix the rate ratio")).tag("fixed")
        }
        if workspace.settings.morphologyRateMode == "estimate" {
          HStack {
            Text(strings("Lognormal log-SD"))
            TextField("1", text: $workspace.settings.morphologyRateLogSD).frame(width: 100)
          }
          Text(
            strings(
              "The prior is log(rate) ~ Normal(0, log-SD²), with median 1. This is a modeling assumption; assess its sensitivity for your data."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).fixedSize(
            horizontal: false, vertical: true)
        } else if workspace.settings.morphologyRateMode == "fixed" {
          HStack {
            Text(strings("Morphology relative rate"))
            TextField("1", text: $workspace.settings.morphologyRate).frame(width: 100)
          }
        }
      }
    }
  }
}
