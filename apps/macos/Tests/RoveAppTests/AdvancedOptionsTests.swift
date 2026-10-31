import Foundation
import Testing

@testable import RoveApp

struct AdvancedOptionsTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }

  @Test func argumentsFollowTheCommandLineAndDefaultsStaySilent() throws {
    var settings = AnalysisSettings()
    let matrix = URL(fileURLWithPath: "/tmp/m.nex")
    let output = URL(fileURLWithPath: "/tmp/out")
    let plain = try settings.arguments(matrix: matrix, output: output)
    for flag in ["--sample-from-prior", "--no-tuning", "--weight", "--tuning", "--min-ess", "--checkpoint-every"] {
      #expect(!plain.contains(flag))
    }
    #expect(!settings.usesAdvancedOptions)
    settings.chains = 2
    settings.stopWhenConverged = true
    settings.minESS = 300
    settings.maxRhat = 1.02
    settings.checkEvery = 5_000
    settings.checkpointEvery = 20_000
    settings.sampleFromPrior = true
    settings.tuning = false
    settings.weights = [MoveOverride(move: "gamma_shape_scale", value: 3)]
    settings.tuningValues = [MoveOverride(move: "subtree_slide", value: 0.00076)]
    #expect(settings.validationError == nil)
    #expect(settings.usesAdvancedOptions)
    let arguments = try settings.arguments(matrix: matrix, output: output)
    let sampling = [
      "--chains", "2", "--stop-when-converged", "--min-ess", "300.0", "--max-rhat", "1.02",
      "--check-every", "5000", "--checkpoint-every", "20000", "--sample-from-prior",
    ]
    let schedule = ["--no-tuning", "--weight", "gamma_shape_scale=3.0", "--tuning", "subtree_slide=0.00076"]
    #expect(arguments.indices.contains { arguments[$0...].starts(with: sampling) })
    #expect(arguments.suffix(schedule.count).elementsEqual(schedule))
    #expect(try settings.modelArguments().suffix(schedule.count).elementsEqual(schedule))
    // Thresholds stay in the setup but are inert while automatic stopping is off.
    settings.stopWhenConverged = false
    #expect(settings.validationError == nil)
    #expect(!(try settings.arguments(matrix: matrix, output: output)).contains("--min-ess"))
    settings.sampleFromPrior = false
    settings.tuning = true
    settings.checkpointEvery = nil
    settings.weights = []
    settings.tuningValues = []
    #expect(!settings.usesAdvancedOptions)
    settings.stopWhenConverged = true
    #expect(settings.usesAdvancedOptions)
  }

  @Test func scheduleOverridesAreCheckedAgainstTheModel() {
    var settings = AnalysisSettings()
    settings.weights = [MoveOverride(move: "unknown_move", value: 1)]
    #expect(settings.validationError != nil)
    settings.weights = [MoveOverride(move: "node_height", value: -1)]
    #expect(settings.validationError != nil)
    settings.weights = [MoveOverride(move: "node_height", value: 2), MoveOverride(move: "node_height", value: 3)]
    #expect(settings.validationError != nil)
    settings.weights = ProposalSchedule.presets[1].weights
    #expect(settings.validationError == nil)
    settings.clock = "strict"
    #expect(settings.validationError != nil)
    settings.weights = [MoveOverride(move: "tip_age", value: 10)]
    #expect(settings.validationError != nil)
    settings.weights = []
    settings.tuningValues = [MoveOverride(move: "tree_scale", value: 1.5)]
    #expect(settings.validationError != nil)
    settings.tuningValues = [MoveOverride(move: "node_height", value: 0.5)]
    #expect(settings.validationError != nil)
    settings.tuningValues = [MoveOverride(move: "subtree_slide", value: 4)]
    #expect(settings.validationError == nil)
    settings.checkpointEvery = 1_500
    #expect(settings.validationError != nil)
    settings.checkpointEvery = 3_000
    #expect(settings.validationError == nil)
    #expect(ProposalSchedule.moves.count == 22 && Set(ProposalSchedule.moves).count == 22)
    #expect(ProposalSchedule.tunable.isSubset(of: ProposalSchedule.moves))
  }

  @Test func datedMovesFollowTheEngineRules() {
    var dating = DatingSettings()
    dating.enabled = true
    dating.clock = .fixed
    func issue(_ move: String) -> String? {
      ProposalSchedule.validate(
        weights: [MoveOverride(move: move, value: 1)], tuningValues: [], clock: "reference",
        dating: dating)
    }
    #expect(issue("birth_difference_scale") != nil && issue("relative_death_scale") != nil)
    #expect(issue("clock_rate_scale") != nil)
    #expect(issue("tip_age") == nil)
    dating.clock = .estimated
    #expect(issue("clock_rate_scale") == nil)
    #expect(issue("fbd_death_scale") != nil)
    dating.estimation.birth.estimated = true
    #expect(issue("fbd_death_scale") != nil && issue("fbd_birth_scale") == nil)
    dating.estimation.estimateSampling = true
    #expect(issue("fbd_rho_logit") == nil)
    dating.enabled = false
    #expect(issue("birth_difference_scale") == nil && issue("fbd_birth_scale") != nil)
  }

  @Test @MainActor func groupRunsVerifyWithScheduleOverrides() async throws {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-advanced-group-\(UUID())")
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: folder) }
    let matrix = folder.appendingPathComponent("matrix.nex")
    try Data(contentsOf: root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"))
      .write(to: matrix)
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = 2_000
    settings.sampleEvery = 100
    settings.chains = 2
    settings.weights = ProposalSchedule.presets[0].weights
    settings.tuningValues = [MoveOverride(move: "tree_scale", value: 0.24)]
    let directory = folder.appendingPathComponent("run")
    let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings)
    let request = RunRequest(id: UUID(), executable: engine, directory: directory, draft: draft)
    #expect(await RunProcess().execute(request) { _ in }.status == .completed)
    let record = try EngineRecord.verify(directory: directory, draft: draft)
    #expect(record.kind == "independent_chains")
    let chain = try EngineRecord.read(
      directory.appendingPathComponent("results/chain-02/run.json"))
    #expect(chain.moves?.first { $0.name == "gamma_shape_scale" }?.weight == 3)
    var changed = settings
    changed.tuningValues = [MoveOverride(move: "tree_scale", value: 0.5)]
    #expect(!chain.matches(AnalysisDraft(matrix: matrix, report: report, settings: changed), seed: chain.settings.seed))
    #expect(HistoryEntry.read(directory).status == .completed)
  }

  @Test func savedSetupsUseFormatFiveOnlyWithAdvancedOptions() async throws {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-advanced-\(UUID())")
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: folder) }
    let matrix = folder.appendingPathComponent("matrix.nex")
    try Data(contentsOf: root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"))
      .write(to: matrix)
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    let plain = AnalysisDraft(matrix: matrix, report: report, settings: settings)
    #expect(plain.formatVersion == 2)
    settings.weights = ProposalSchedule.presets[0].weights
    let advanced = AnalysisDraft(matrix: matrix, report: report, settings: settings)
    #expect(advanced.formatVersion == 5)
    let file = folder.appendingPathComponent("setup.rove.json")
    try advanced.write(to: file)
    let reopened = try AnalysisDraft.read(file)
    #expect(reopened.settings.weights == settings.weights)
    #expect(reopened.settings.tuning && reopened.settings.minESS == nil)
    // A format-4 file carrying advanced options is rejected: older apps would ignore them.
    var text = try String(contentsOf: file, encoding: .utf8)
    text = text.replacingOccurrences(of: "\"formatVersion\" : 5", with: "\"formatVersion\" : 4")
    try text.write(to: file, atomically: true, encoding: .utf8)
    #expect(throws: (any Error).self) { try AnalysisDraft.read(file) }
  }

  @Test @MainActor func runsRecordTheOptionsAndSummariesAcceptSampleBurnin() async throws {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-advanced-run-\(UUID())")
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: folder) }
    let matrix = folder.appendingPathComponent("matrix.nex")
    try Data(contentsOf: root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"))
      .write(to: matrix)
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = 2_000
    settings.sampleEvery = 100
    settings.sampleFromPrior = true
    settings.tuning = false
    settings.checkpointEvery = 1_000
    settings.weights = [MoveOverride(move: "gamma_shape_scale", value: 3)]
    settings.tuningValues = [MoveOverride(move: "tree_scale", value: 0.24)]
    let directory = folder.appendingPathComponent("run")
    let request = RunRequest(
      id: UUID(), executable: engine, directory: directory,
      draft: AnalysisDraft(
        matrix: matrix, report: report, settings: settings,
        monophyly: [Array(report.allTaxa.prefix(19))]))
    #expect(await RunProcess().execute(request) { _ in }.status == .completed)
    let entry = HistoryEntry.read(directory)
    #expect(entry.status == .completed)
    let record = try EngineRecord.read(directory.appendingPathComponent("results/run.json"))
    #expect(record.model.sampleFromPrior == true && record.settings.tuning == false)
    #expect(record.moves?.first { $0.name == "gamma_shape_scale" }?.weight == 3)
    #expect(record.moves?.first { $0.name == "tree_scale" }?.initialTuning == 0.24)
    var changed = settings
    changed.weights = [MoveOverride(move: "gamma_shape_scale", value: 0.1)]
    #expect(!record.matches(AnalysisDraft(matrix: matrix, report: report, settings: changed, monophyly: request.draft.monophyly)))
    changed = settings
    changed.sampleFromPrior = false
    #expect(!record.matches(AnalysisDraft(matrix: matrix, report: report, settings: changed, monophyly: request.draft.monophyly)))
    #expect(entry.draft?.settings.advancedRows(strings: AppStrings(language: .english)).count == 5)

    let summary = TreeSummaryModel(directory: directory, executable: engine)
    summary.burninUnit = "samples"
    summary.burnin = "7"
    #expect(summary.valid)
    summary.start()
    await summary.waitForExit()
    #expect(summary.error == nil, Comment(rawValue: summary.details ?? ""))
    let saved = try #require(summary.summaries.first)
    #expect(saved.record.discardedSamples == 7 && saved.record.retainedSamples == 14)
    summary.burnin = "150"
    #expect(summary.valid)
    summary.burninUnit = "percent"
    #expect(!summary.valid)
  }
}
