import Foundation
import Testing

@testable import RoveApp

struct ClockSigmaPriorTests {
  @Test func priorSettingsRoundTripAndLegacyDefaults() throws {
    var settings = AnalysisSettings()
    #expect(settings.clockSigmaPrior == .reference)
    settings.clockSigmaShape = "2.5"
    settings.clockSigmaScale = "0,4"
    #expect(settings.validationError == nil)
    #expect(settings.usesCustomSigmaPrior)
    let encoded = try JSONEncoder().encode(settings)
    let restored = try JSONDecoder().decode(AnalysisSettings.self, from: encoded)
    #expect(restored == settings)
    #expect(
      try restored.modelArguments().suffix(4) == [
        "--clock-sigma-shape", "2.5", "--clock-sigma-scale", "0.4",
      ])
    var legacy = try #require(JSONSerialization.jsonObject(with: encoded) as? [String: Any])
    legacy.removeValue(forKey: "clockSigmaShape")
    legacy.removeValue(forKey: "clockSigmaScale")
    let old = try JSONDecoder().decode(
      AnalysisSettings.self, from: JSONSerialization.data(withJSONObject: legacy))
    #expect(old.clockSigmaPrior == .reference)
    #expect(!old.usesCustomSigmaPrior)
    for value in ["", "0", "-1", "nan", "inf", "1e309"] {
      settings.clockSigmaScale = value
      #expect(settings.validationError != nil)
      #expect(throws: (any Error).self) { try settings.modelArguments() }
    }
    settings.clock = "strict"
    #expect(settings.validationError == nil)
    #expect(!(try settings.modelArguments()).contains("--clock-sigma-shape"))
  }

  @Test func customPriorRequiresNewSetupVersionAndMatchesEngineRecord() async throws {
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let engine =
      ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.clockSigmaShape = "2.5"
    settings.clockSigmaScale = "0.4"
    let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings)
    #expect(draft.formatVersion == 6)
    try draft.validate()
    let bytes = try JSONEncoder().encode(draft)
    var json = try #require(JSONSerialization.jsonObject(with: bytes) as? [String: Any])
    json["formatVersion"] = 5
    let downgraded = try JSONDecoder().decode(
      AnalysisDraft.self, from: JSONSerialization.data(withJSONObject: json))
    #expect(throws: (any Error).self) { try downgraded.validate() }
    let model = """
      {"monophyly":[],"preset":"reference","clock_sigma_prior_parameters":{"shape":2.5,"scale":0.4}}
      """
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let record = try decoder.decode(EngineRecord.Model.self, from: Data(model.utf8))
    #expect(record.clockSigmaPriorParameters == settings.clockSigmaPrior)
    #expect(record.clockSigmaPriorParameters != AnalysisSettings().clockSigmaPrior)

    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: folder) }
    settings.steps = 200
    settings.sampleEvery = 10
    settings.chains = 2
    settings.sampleFromPrior = true
    let fitted = AnalysisDraft(matrix: matrix, report: report, settings: settings)
    let request = RunRequest(id: UUID(), executable: engine, directory: folder, draft: fitted)
    let result = await RunProcess().execute(request) { _ in }
    #expect(result.status == .completed, Comment(rawValue: "\(result)"))
    let saved = try EngineRecord.verify(directory: folder, draft: fitted)
    #expect(saved.model.clockSigmaPriorParameters == settings.clockSigmaPrior)
    settings.clockSigmaShape = "5"
    settings.clockSigmaScale = "0.2"
    let wrong = AnalysisDraft(matrix: matrix, report: report, settings: settings)
    #expect(!saved.matches(wrong))
  }
}
