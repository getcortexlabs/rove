import Foundation

struct SamplingPreset: Identifiable, Sendable {
  let millions: Int
  var id: Int { steps }
  var steps: Int { millions * 1_000_000 }
  var sampleEvery: Int { millions * 100 }
  static let all = [1, 10, 20, 30, 50, 100].map { SamplingPreset(millions: $0) }
}

extension AnalysisSettings {
  var samplingPreset: SamplingPreset? {
    SamplingPreset.all.first { $0.steps == steps && $0.sampleEvery == sampleEvery }
  }

  mutating func apply(_ preset: SamplingPreset) {
    steps = preset.steps
    sampleEvery = preset.sampleEvery
  }

  /// Explicit intervals are never changed by restoring a setup or selecting a preset.
  /// The setup screen offers these adjustments separately, before launch.
  var samplingIntervalAdjustments: [(label: String, value: Int)] {
    guard sampleEvery > 0 else { return [] }
    return [("Diagnostic interval", checkEvery), ("Checkpoint interval", checkpointEvery)]
      .compactMap { label, value in
        guard let value, value > 0, value % sampleEvery != 0 else { return nil }
        let (rounded, overflow) = value.addingReportingOverflow(sampleEvery - value % sampleEvery)
        return overflow ? nil : (label, rounded)
      }
  }

  mutating func alignSamplingIntervals() {
    for adjustment in samplingIntervalAdjustments {
      if adjustment.label == "Diagnostic interval" { checkEvery = adjustment.value }
      if adjustment.label == "Checkpoint interval" { checkpointEvery = adjustment.value }
    }
  }
}
