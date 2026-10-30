import Foundation
import Testing

@testable import RoveApp

struct DatingTests {
  static var example: DatingSettings {
    var dating = DatingSettings()
    dating.enabled = true
    dating.birthRate = "0,08"
    dating.deathRate = "0.03"
    dating.fossilSamplingRate = "2e-2"
    dating.presentSampling = "0.7"
    dating.origin = "50"
    dating.clockRate = "0.01"
    dating.clockPriorMean = "0.01"
    dating.ages = [
      "Living A": TipAgeInput(), "Living C": TipAgeInput(),
      "Fossil B": TipAgeInput(kind: .interval, minimum: "5", maximum: "9"),
      "Fossil D": TipAgeInput(kind: .fixed, fixed: "12"),
    ]
    return dating
  }
  static var estimatedExample: DatingSettings {
    var dating = example
    dating.estimation.birth = ProcessRateInput(estimated: true, mean: "0.08")
    dating.estimation.death = ProcessRateInput(estimated: true, mean: "0.03")
    dating.estimation.fossilSampling = ProcessRateInput(estimated: true, mean: "0.02")
    dating.estimation.estimateSampling = true
    dating.estimation.samplingAlpha = "2"
    dating.estimation.samplingBeta = "2"
    dating.estimation.origin = ProcessRateInput(estimated: true, mean: "30")
    return dating
  }

  @Test func estimatedInputsHaveExplicitPriorsAndModelSemantics() throws {
    let valid = Self.estimatedExample
    #expect(valid.validationError() == nil)
    let encoded = try #require(try valid.model())
    #expect(encoded.processModel == "joint_sampling_v1")
    #expect(encoded.processPriors?.presentSamplingBeta == [2, 2])
    #expect(try EngineDatingModel.decode(encoded.json()) == encoded)
    let changes: [(inout DatingSettings) -> Void] = [
      { $0.estimation.birth.mean = "" }, { $0.estimation.origin.mean = "-1" },
      { $0.estimation.samplingAlpha = "0" }, { $0.estimation.samplingBeta = "1001" },
      { $0.deathRate = "0" }, { $0.presentSampling = "1" },
    ]
    for change in changes {
      var invalid = valid
      change(&invalid)
      #expect(invalid.validationError() != nil)
      #expect(throws: (any Error).self) { try invalid.model() }
    }
    var mixed = valid
    mixed.estimation.death.estimated = false
    mixed.deathRate = "0"
    #expect(mixed.validationError() == nil)
    #expect(try mixed.model()?.processPriors?.deathRateMean == nil)
  }

  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }
  private var matrix: URL { root.appendingPathComponent("fixtures/validation/dating/matrix.nex") }

  @Test func invalidEditsCannotReusePreviousValuesOrChangeTheModelSilently() throws {
    let valid = Self.example
    #expect(valid.validationError(taxa: Array(valid.ages.keys)) == nil)
    #expect(try valid.model()?.birthRate == 0.08)
    #expect(try valid.model()?.fossilSamplingRate == 0.02)
    #expect(valid.validationError(taxa: ["Unknown"]) != nil)
    let changes: [(inout DatingSettings) -> Void] = [
      { $0.birthRate = "" }, { $0.deathRate = "-1" }, { $0.fossilSamplingRate = "0" },
      { $0.origin = "12" }, { $0.presentSampling = "1.1" }, { $0.presentSampling = "0" },
      { $0.clockRate = "NaN" }, { $0.clockPriorMean = "inf" }, { $0.birthRate = "0,1.2" },
      { $0.timeUnit = "\n" }, { $0.timeUnit = String(repeating: "a", count: 81) },
      { $0.ages["Fossil B"]?.maximum = "5" }, { $0.ages["Fossil B"]?.minimum = "0" },
      { $0.ages["Fossil D"]?.fixed = "-12" }, { $0.ages = [:] },
      {
        $0.deathRate = "0"
        $0.presentSampling = "1"
      },
    ]
    for change in changes {
      var edited = valid
      change(&edited)
      #expect(edited.validationError() != nil)
      #expect(throws: (any Error).self) { try edited.model() }
    }
    var fixed = valid
    fixed.clock = .fixed
    fixed.clockPriorMean = ""
    #expect(fixed.validationError() == nil)
    let model = try #require(try fixed.model())
    let json = try model.json()
    #expect(!json.contains("clock_prior_mean"))
    #expect(try EngineDatingModel.decode(json) == fixed.model())
    fixed.enabled = false
    fixed.birthRate = "unfinished"
    #expect(fixed.validationError() == nil)
    #expect(try fixed.model() == nil)
  }

  @Test @MainActor func datingSurvivesSetupReuseAndLegacySetupsRemainUndated() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    let suite = "rove-dating-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer {
      defaults.removePersistentDomain(forName: suite)
      try? FileManager.default.removeItem(at: directory)
    }
    let workspace = Workspace(executable: engine, preferences: AppPreferences(defaults: defaults))
    workspace.load(matrix)
    while workspace.loading { try await Task.sleep(for: .milliseconds(10)) }
    let report = try #require(workspace.report)
    workspace.setDatingEnabled(true)
    #expect(Set(workspace.settings.dating.ages.keys) == Set(report.allTaxa))
    #expect(workspace.settings.dating.fossilCount == 0 && workspace.issue != nil)
    workspace.settings.dating = Self.example
    #expect(workspace.issue == nil)
    workspace.setDatingEnabled(false)
    #expect(
      !(try workspace.settings.arguments(matrix: matrix, output: directory)).contains(
        "--dating-json"))
    workspace.setDatingEnabled(true)
    #expect(workspace.settings.dating == Self.example)
    let draft = AnalysisDraft(matrix: matrix, report: report, settings: workspace.settings)
    let saved = directory.appendingPathComponent("setup.rove.json")
    try draft.write(to: saved)
    #expect(try AnalysisDraft.read(saved).settings.dating == Self.example)
    #expect(draft.formatVersion == 3)
    var estimated = workspace.settings
    estimated.dating = Self.estimatedExample
    let newer = AnalysisDraft(matrix: matrix, report: report, settings: estimated)
    #expect(newer.formatVersion == 4)
    let newerFile = directory.appendingPathComponent("estimated.rove.json")
    try newer.write(to: newerFile)
    #expect(try AnalysisDraft.read(newerFile).settings.dating == Self.estimatedExample)
    var oldVersion = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: newerFile)) as? [String: Any])
    oldVersion["formatVersion"] = 3
    try JSONSerialization.data(withJSONObject: oldVersion).write(to: newerFile)
    #expect(throws: (any Error).self) { try AnalysisDraft.read(newerFile) }

    workspace.loadSetup(saved)
    while workspace.loading { try await Task.sleep(for: .milliseconds(10)) }
    #expect(workspace.settings.dating == Self.example)
    var object = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: saved)) as? [String: Any])
    object["formatVersion"] = 2
    try JSONSerialization.data(withJSONObject: object).write(to: saved)
    #expect(throws: (any Error).self) { try AnalysisDraft.read(saved) }
    var settings = try #require(object["settings"] as? [String: Any])
    settings.removeValue(forKey: "dating")
    object["settings"] = settings
    try JSONSerialization.data(withJSONObject: object).write(to: saved)
    #expect(try !AnalysisDraft.read(saved).settings.dating.enabled)
    workspace.load(root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"))
    while workspace.loading { try await Task.sleep(for: .milliseconds(10)) }
    #expect(!workspace.settings.dating.enabled && workspace.settings.dating.ages.isEmpty)
  }

  @Test(arguments: [false, true]) @MainActor
  func appRunsPreserveDatingInHistoryDiagnosticsAndTreeUnits(estimated: Bool) async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-dated-\(UUID())")
    defer { try? FileManager.default.removeItem(at: parent) }
    let suite = "rove-dating-history-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let workspace = Workspace(executable: engine, preferences: AppPreferences(defaults: defaults))
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    for clock in ["strict", "reference"] {
      var settings = AnalysisSettings()
      settings.clock = clock
      settings.steps = 10_000
      settings.sampleEvery = 100
      settings.chains = clock == "strict" ? 1 : 2
      settings.dating = estimated ? Self.estimatedExample : Self.example
      settings.dating.clock = clock == "strict" ? .estimated : .fixed
      let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings)
      let request = RunRequest(
        id: UUID(), executable: engine, directory: parent.appendingPathComponent(clock),
        draft: draft)
      let result = await RunProcess().execute(request) { _ in }
      #expect(result.status == .completed, "\(String(describing: result))")
      let history = HistoryEntry.read(request.directory)
      #expect(history.status == .completed && history.draft?.settings.dating == settings.dating)
      workspace.reuseSetup(history)
      while workspace.loading { try await Task.sleep(for: .milliseconds(10)) }
      #expect(workspace.settings.dating == settings.dating && workspace.issue == nil)
      let record = try EngineRecord.verify(directory: request.directory, draft: draft)
      #expect(record.model.dating == (try settings.dating.model()))
      var wrong = settings
      wrong.dating.ages["Fossil D"]?.fixed = "13"
      #expect(!record.matches(AnalysisDraft(matrix: matrix, report: report, settings: wrong)))
      let output = request.output.appendingPathComponent(settings.chains == 1 ? "" : "chain-01")
      let trace = try String(
        contentsOf: output.appendingPathComponent("trace.log"), encoding: .utf8)
      #expect(trace.contains("fossilized_birth_death") && trace.contains("tip_age_4"))
      if settings.chains > 1 {
        let comparison = try ChainComparison.read(
          request.output.appendingPathComponent("comparison.json"))
        #expect(comparison.statistics.first { $0.name == "clock_rate" }?.fixedByModel == true)
        #expect(comparison.statistics.first { $0.name == "tip_age_2" }?.fixedByModel != true)
      }
      let runDirectory = settings.chains == 1 ? request.directory : output
      let summary = TreeSummaryModel(directory: runDirectory, executable: engine)
      summary.start()
      await summary.waitForExit()
      #expect(summary.error == nil, "\(String(describing: summary.error))")
      let saved = try #require(summary.summaries.first)
      let explorer = TreeExplorer(
        directory: saved.directory, runDirectory: runDirectory, executable: engine)
      await explorer.load()
      let tree = try #require(explorer.document)
      #expect(tree.summary.timeUnit == "Ma")
      #expect(tree.nodes.first { $0.taxon == "Fossil D" }?.height == 12)
      let figure = TreeFigure(
        tree: tree, layout: TreeLayout.build(tree, collapsed: [], rotated: []),
        viewport: CGSize(width: 700, height: 500), collapsed: [], selected: nil,
        showSupport: true, strings: AppStrings(language: .english))
      #expect(figure.scaleLabel.hasSuffix(" Ma"))
      #expect(figure.svg(runName: "Dated example").contains(" Ma</text>"))
    }
  }

  @Test(arguments: [false, true]) func datedCheckpointsResumeWithoutTheConfigurationFile(
    estimated: Bool
  ) async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-dated-resume-\(UUID())")
    defer { try? FileManager.default.removeItem(at: parent) }
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    for chains in [1, 2] {
      var settings = AnalysisSettings()
      settings.steps = 100_000
      settings.sampleEvery = 100
      settings.chains = chains
      settings.dating = estimated ? Self.estimatedExample : Self.example
      let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings)
      let directory = parent.appendingPathComponent("paused-\(chains)")
      let request = RunRequest(id: UUID(), executable: engine, directory: directory, draft: draft)
      let worker = RunProcess()
      let result = await worker.execute(request) { event in
        if event.event == "started" { worker.pause() }
      }
      #expect(result.status == .paused, "\(String(describing: result))")
      let checkpoint = try RunCheckpoint.read(directory: request.directory, draft: draft)
      #expect(checkpoint.step < UInt64(settings.steps))
      var changed = settings
      changed.dating.origin = "51"
      #expect(throws: (any Error).self) {
        try RunCheckpoint.read(
          directory: request.directory,
          draft: AnalysisDraft(matrix: matrix, report: report, settings: changed))
      }
      try FileManager.default.removeItem(at: request.datingFile)
      let continued = await RunProcess().execute(
        RunRequest(
          id: UUID(), executable: engine,
          directory: directory, draft: draft, resume: true)
      ) { _ in }
      #expect(continued.status == .completed, "\(String(describing: continued))")
      let full = RunRequest(
        id: UUID(), executable: engine,
        directory: parent.appendingPathComponent("full-\(chains)"), draft: draft)
      #expect(await RunProcess().execute(full) { _ in }.status == .completed)
      for chain in 1...chains {
        let folder = chains == 1 ? "" : String(format: "chain-%02d", chain)
        for file in ["trace.log", "trees.nexus"] {
          func samples(_ request: RunRequest) throws -> [Substring] {
            try String(
              contentsOf: request.output.appendingPathComponent(folder).appendingPathComponent(
                file), encoding: .utf8
            )
            .split(separator: "\n").filter {
              !$0.hasPrefix("# command") && !$0.hasPrefix("[command")
            }
          }
          #expect(try samples(request) == samples(full))
        }
      }
    }
  }
}
