import Foundation
import Testing

@testable import RoveApp

struct SamplingPlanTests {
  @Test func presetsReachTheRequestedLengthWithTenThousandDraws() throws {
    for preset in SamplingPreset.all {
      var settings = AnalysisSettings()
      settings.apply(preset)
      #expect(settings.validationError == nil)
      #expect(settings.steps / settings.sampleEvery + 1 == 10_001)
      let arguments = try settings.arguments(
        matrix: URL(fileURLWithPath: "/matrix.nex"), output: URL(fileURLWithPath: "/output"))
      #expect(
        arguments[try #require(arguments.firstIndex(of: "--steps")) + 1] == String(preset.steps))
      #expect(
        arguments[try #require(arguments.firstIndex(of: "--sample-every")) + 1]
          == String(preset.sampleEvery))
    }
  }

  @Test func savedCustomIntervalsArePreservedAndPresetsCanBeReapplied() throws {
    var settings = AnalysisSettings()
    let preset = SamplingPreset(millions: 30)
    settings.apply(preset)
    settings.sampleEvery = 500
    #expect(settings.samplingPreset == nil)
    let restored = try JSONDecoder().decode(
      AnalysisSettings.self, from: JSONEncoder().encode(settings))
    #expect(restored == settings)
    #expect(restored.sampleEvery == 500)
    settings.apply(preset)
    #expect(settings.samplingPreset?.millions == 30)
    #expect(settings.sampleEvery == 3_000)
  }

  @Test func advancedIntervalsNeedAnExplicitAdjustment() throws {
    var settings = AnalysisSettings()
    settings.checkEvery = 10_000
    settings.checkpointEvery = 50_000
    settings.apply(SamplingPreset(millions: 30))
    #expect(settings.validationError != nil)
    #expect(settings.checkEvery == 10_000 && settings.checkpointEvery == 50_000)
    #expect(settings.samplingIntervalAdjustments.map(\.value) == [12_000, 51_000])
    settings.alignSamplingIntervals()
    #expect(settings.validationError == nil)
    #expect(settings.checkEvery == 12_000 && settings.checkpointEvery == 51_000)
    settings.checkpointEvery = Int.max
    #expect(settings.samplingIntervalAdjustments.isEmpty)
    #expect(settings.validationError != nil)
  }
}
