import Foundation
import Testing

@testable import RoveApp

struct TreePriorTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }

  @Test func modelAndProposalValidation() throws {
    var settings = AnalysisSettings()
    #expect(!((try settings.modelArguments()).contains("--tree-prior")))
    settings.treePrior = "yule"
    settings.birthRateUpper = "2,5"
    #expect(settings.validationError == nil && settings.yuleBirthRateUpper == 2.5)
    #expect((try settings.modelArguments()).contains("--birth-rate-upper"))
    for name in ["birth_difference_scale", "relative_death_scale"] {
      settings.tuningValues = [MoveOverride(move: name, value: 0.5)]
      #expect(settings.validationError != nil)
    }
    settings.tuningValues = [MoveOverride(move: "birth_rate_scale", value: 0.5)]
    #expect(settings.validationError == nil)
    settings.treePrior = "birth-death"
    #expect(settings.validationError != nil)
    settings.treePrior = "yule"
    for value in ["", "0", "-1", "nan", "inf", "1e-999"] {
      settings.birthRateUpper = value
      #expect(settings.validationError != nil)
    }
    settings.birthRateUpper = "2.5"
    settings.dating.enabled = true
    #expect(settings.validationError?.contains("contemporaneous") == true)
    #expect(throws: (any Error).self) { try settings.modelArguments() }
  }

  @Test @MainActor func yulePersistsResumesPredictsAndEstimatesItsFullModel() async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-yule-\(UUID())")
    try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: parent) }
    let matrix = parent.appendingPathComponent("matrix.nex")
    try "#NEXUS BEGIN DATA; DIMENSIONS NTAX=4 NCHAR=4; MATRIX A 0011 B 0101 C 1010 D 1100; END;"
      .write(to: matrix, atomically: true, encoding: .utf8)
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    for clock in ["strict", "reference"] {
      var settings = AnalysisSettings()
      settings.treePrior = "yule"
      settings.birthRateUpper = "4"
      settings.clock = clock
      settings.steps = 2_000
      settings.sampleEvery = 100
      settings.chains = clock == "strict" ? 1 : 2
      settings.tuningValues = [MoveOverride(move: "birth_rate_scale", value: 0.6)]
      let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings, outgroup: "D")
      #expect(draft.formatVersion == 10)
      let file = parent.appendingPathComponent("setup-\(clock).rove.json")
      try draft.write(to: file)
      #expect(try AnalysisDraft.read(file).settings == settings)
      var object = try #require(
        JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any])
      object["formatVersion"] = 9
      try JSONSerialization.data(withJSONObject: object).write(to: file)
      #expect(throws: (any Error).self) { try AnalysisDraft.read(file) }

      let directory = parent.appendingPathComponent(clock)
      let request = RunRequest(id: UUID(), executable: engine, directory: directory, draft: draft)
      let worker = RunProcess()
      let paused = await worker.execute(request) { event in
        if event.event == "started" { worker.pause() }
      }
      #expect(paused.status == .paused)
      #expect(HistoryEntry.read(directory).checkpoint != nil)
      let resumed = RunRequest(
        id: UUID(), executable: engine, directory: directory, draft: draft, resume: true)
      #expect(await RunProcess().execute(resumed) { _ in }.status == .completed)
      let record = try EngineRecord.verify(directory: directory, draft: draft)
      #expect(
        record.model.treePriorKind == "yule" && record.model.birthRatePriorParameters?.upper == 4)
      let entry = HistoryEntry.read(directory)
      #expect(entry.draft?.settings.treePrior == "yule")
      var changed = settings
      changed.birthRateUpper = "5"
      #expect(
        !record.matches(
          AnalysisDraft(matrix: matrix, report: report, settings: changed, outgroup: "D")))

      let result = directory.appendingPathComponent(
        settings.chains == 1 ? "results" : "results/chain-01")
      let trace = try String(
        contentsOf: result.appendingPathComponent("trace.log"), encoding: .utf8)
      #expect(trace.contains("\tyule\tbirth_rate\t") && !trace.contains("relative_death"))
      let child = try EngineRecord.read(result.appendingPathComponent("run.json"))
      #expect(child.moves?.contains(where: { $0.name == "birth_rate_scale" }) == true)
      #expect(child.moves?.contains(where: { $0.name == "relative_death_scale" }) == false)

      let imported = try await CommandLineImport.prepare(
        directory.appendingPathComponent("results"), matrix: matrix, under: parent,
        executable: engine)
      #expect(HistoryEntry.read(imported).draft?.settings.treePrior == "yule")
      let prediction = PredictiveCheckModel(entry: entry, executable: engine)
      prediction.replicates = "2"
      prediction.start()
      await prediction.waitForExit()
      #expect(prediction.error == nil, Comment(rawValue: prediction.details ?? ""))
      #expect(prediction.predictions.first?.report.replicates == 2)

      let marginal = MarginalLikelihoodModel(entry: entry, executable: engine)
      marginal.powers = "4"
      marginal.steps = "400"
      marginal.start()
      await marginal.waitForExit()
      #expect(marginal.error == nil, Comment(rawValue: marginal.details ?? ""))
      let estimate = try #require(marginal.estimates.first)
      #expect(estimate.matches(draft) && estimate.record.model.treePriorKind == "yule")
      #expect(
        !estimate.matches(
          AnalysisDraft(matrix: matrix, report: report, settings: changed, outgroup: "D")))
    }
  }
}
