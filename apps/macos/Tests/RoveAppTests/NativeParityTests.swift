import Foundation
import Testing

@testable import RoveApp

struct NativeParityTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL { root.appendingPathComponent("target/release/rove") }

  @Test @MainActor func customPlanRunsThreeChainsAndImportsTheCLIResults() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-parity-\(UUID())")
    defer { try? FileManager.default.removeItem(at: directory) }
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = 600
    settings.sampleEvery = 10
    settings.chains = 3
    settings.comparisonBurnin = 25
    settings.backend = "portable"
    settings.printEvery = 50
    let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings)
    #expect(draft.formatVersion == 7)
    let request = RunRequest(id: UUID(), executable: engine, directory: directory, draft: draft)
    #expect(try request.arguments.filter { $0 == "--print-every" }.count == 1)
    let run = AnalysisRun(request: request)
    run.start()
    await run.waitForExit()
    #expect(run.status == .completed, "\(run.result?.error ?? "unknown run error")")
    let comparison = try ChainComparison.read(
      request.output.appendingPathComponent("comparison.json"))
    #expect(comparison.chainCount == 3 && comparison.burninPercent == 25)
    let checkpoint = try RunCheckpoint.read(directory: directory, draft: draft)
    #expect(checkpoint.chains.count == 3 && checkpoint.step == 600)
    let imported = try await CommandLineImport.prepare(
      request.output, matrix: matrix,
      under: directory.appendingPathComponent("imports"), executable: engine)
    let entry = HistoryEntry.read(imported)
    #expect(
      entry.status == .completed && entry.draft?.settings.chains == 3
        && entry.draft?.settings.comparisonBurnin == 25
        && entry.draft?.settings.backend == "portable")
    #expect(
      FileManager.default.fileExists(
        atPath: request.output.appendingPathComponent("trace.log").path) == false)
    let wrong = root.appendingPathComponent("fixtures/validation/ordered/matrix.nex")
    await #expect(throws: (any Error).self) {
      try await CommandLineImport.prepare(
        request.output, matrix: wrong, under: directory, executable: engine)
    }
    let saved = directory.appendingPathComponent("setup.rove.json")
    var json = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: saved)) as? [String: Any])
    json["formatVersion"] = 6
    try JSONSerialization.data(withJSONObject: json).write(to: saved)
    #expect(throws: (any Error).self) { try AnalysisDraft.read(saved) }
  }

  @Test @MainActor func externalTraceSupportsSampleBurninAndPersistsTheUnit() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-trace-import-\(UUID())")
    defer { try? FileManager.default.removeItem(at: directory) }
    let source = root.appendingPathComponent("fixtures/diagnostics/synthetic.tsv")
    let original = try Data(contentsOf: source)
    let model = TraceDiagnosticsModel(directory: directory, executable: engine, source: source)
    model.burninUnit = "samples"
    model.burnin = "300"
    model.start()
    await model.waitForExit()
    let result = try #require(model.result)
    #expect(result.report.discardedSamples == 300 && result.report.burnin.unit == "samples")
    let reopened = TraceDiagnosticsModel(directory: directory, executable: engine, source: source)
    await reopened.load()
    #expect(reopened.burnin == "300" && reopened.burninUnit == "samples")
    #expect(try Data(contentsOf: source) == original)
    model.burnin = String(result.report.totalSamples)
    model.start()
    await model.waitForExit()
    #expect(model.error != nil)
  }

  @Test func allSupportedChainCountsAndInvalidExecutionOptions() {
    var settings = AnalysisSettings()
    for count in 1...16 {
      settings.chains = count
      #expect(settings.validationError == nil)
    }
    settings.chains = 17
    #expect(settings.validationError != nil)
    settings.chains = 1
    settings.comparisonBurnin = 100
    #expect(settings.validationError != nil)
    settings.comparisonBurnin = 10
    settings.printEvery = 0
    #expect(settings.validationError != nil)
    settings.printEvery = nil
    settings.backend = "gpu"
    #expect(settings.validationError != nil)
  }

  @Test func importsDefaultCLIOptionsWithoutInventingNativeDefaults() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-cli-defaults-\(UUID())")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let source = directory.appendingPathComponent("cli")
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let outcome = try await EngineTool().execute(
      executable: engine,
      arguments: [
        "sample", matrix.path, "--output", source.path, "--seed", "42", "--steps", "100",
        "--sample-every", "10",
      ]
    ) { _ in }
    #expect(outcome.status == 0)
    let imported = try await CommandLineImport.prepare(
      source, matrix: matrix, under: directory, executable: engine)
    let entry = HistoryEntry.read(imported)
    #expect(entry.status == .completed)
    #expect(entry.draft?.settings.clock == "strict")
    #expect(entry.draft?.settings.steps == 100 && entry.draft?.settings.sampleEvery == 10)
  }

  @Test @MainActor func modelEvidenceNeedsNoEarlierRunAndHasItsOwnSamplingPlan() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-evidence-\(UUID())")
    defer { try? FileManager.default.removeItem(at: directory) }
    let suite = "rove-evidence-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let history = RunHistory(defaults: defaults, defaultRoot: directory)
    let workspace = Workspace(
      executable: engine, preferences: AppPreferences(defaults: defaults), history: history)
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    workspace.file = matrix
    workspace.report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, executable: engine)
    workspace.outputParent = directory
    workspace.page = .review
    workspace.settings.clock = "strict"
    workspace.prepareModelComparison()
    let path = try #require(workspace.selectedHistoryID)
    let entry = HistoryEntry.read(URL(fileURLWithPath: path))
    #expect(entry.status == .modelSetup)
    let model = MarginalLikelihoodModel(entry: entry, executable: engine)
    #expect(model.available)
    model.powers = "3"
    model.steps = "300"
    model.samplingInterval = "10"
    model.burnin = "10"
    model.start()
    await model.waitForExit()
    #expect(model.error == nil, Comment(rawValue: model.details ?? ""))
    let saved = try #require(model.estimates.first)
    let draft = try #require(entry.draft)
    #expect(saved.estimate.settings.sampleEvery == 10 && saved.matches(draft))
    #expect(
      !FileManager.default.fileExists(
        atPath: entry.directory.appendingPathComponent("results").path))
    let reopened = MarginalLikelihoodModel(
      entry: HistoryEntry.read(entry.directory), executable: engine)
    await reopened.load(history: [entry])
    #expect(reopened.estimates.count == 1)
  }
}
