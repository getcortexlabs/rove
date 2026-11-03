import Foundation

struct MixedInputMetadata: Codable, Equatable, Sendable {
  struct Source: Codable, Equatable, Sendable {
    let name: String
    let sha256: String
    let dataType: String
    let missingTaxa: [String]
  }
  let missingTaxaPolicy: String
  let taxonMatching: String
  let sources: [Source]

  var valid: Bool {
    ["reject", "missing"].contains(missingTaxaPolicy)
      && sources.map(\.dataType) == ["standard", "dna"]
      && sources.allSatisfy { !$0.name.isEmpty && $0.sha256.count == 64 }
  }
}

struct MixedModel: Decodable, Equatable, Sendable {
  struct Prior: Decodable, Equatable, Sendable {
    let distribution: String
    let logMean: Double
    let logSd: Double
  }
  let topology: String
  let branchClock: String
  let gammaShape: String
  let dnaRate: Double
  let morphologyRate: Double
  let morphologyRatePrior: Prior?
  let morphologyConditioning: String
  let dnaConditioning: String

  func matches(_ settings: AnalysisSettings) -> Bool {
    guard topology == "shared", branchClock == "shared", gammaShape == "shared", dnaRate == 1,
      dnaConditioning == "none", morphologyConditioning == settings.conditioning
    else { return false }
    if settings.morphologyRateMode == "estimate" {
      guard let prior = morphologyRatePrior else { return false }
      return morphologyRate == 1 && prior.distribution == "lognormal" && prior.logMean == 0
        && prior.logSd == settings.morphologyLogSDValue
    }
    return settings.morphologyRateMode == "fixed" && morphologyRatePrior == nil
      && morphologyRate == settings.morphologyRateValue
  }
}

extension AnalysisSettings {
  private func positive(_ text: String) -> Double? {
    guard let value = Double(text.replacingOccurrences(of: ",", with: ".")), value.isFinite,
      value > 0
    else { return nil }
    return value
  }
  var morphologyConditioningLabel: String {
    conditioning == "variable"
      ? "Mkv · variable characters"
      : conditioning == "informative"
        ? "Mkparsinf · informative characters" : "Mk · all characters"
  }
  var morphologyRateValue: Double? { positive(morphologyRate) }
  var morphologyLogSDValue: Double? { positive(morphologyRateLogSD) }
  var mixedRateError: String? {
    guard dataType == "mixed" else { return nil }
    guard ["estimate", "fixed"].contains(morphologyRateMode) else {
      return "Choose whether to fix or estimate the morphology/DNA rate ratio."
    }
    guard
      morphologyRateMode == "estimate" ? morphologyLogSDValue != nil : morphologyRateValue != nil
    else { return "Morphology rate and lognormal log-SD must be positive and finite." }
    return nil
  }
  func mixedRateDescription(strings: AppStrings) -> String {
    if morphologyRateMode == "estimate" {
      return strings(
        "Estimated · log(rate) Normal(0, %@²)",
        morphologyLogSDValue.map(String.init(describing:)) ?? "—")
    }
    return strings(
      "Fixed morphology/DNA ratio: %@", morphologyRateValue.map(String.init(describing:)) ?? "—")
  }
}
