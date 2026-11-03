import Foundation
import Testing

@testable import RoveApp

struct MixedTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }
  @MainActor private func wait(_ workspace: Workspace) async throws {
    for _ in 0..<1000 {
      if !workspace.loading { return }
      try await Task.sleep(for: .milliseconds(10))
    }
    throw ImportError.message("Mixed import did not finish")
  }

  @Test @MainActor func combinedSourcesSettingsAndValidationPersist() async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-mixed-import-\(UUID())")
    try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: parent) }
    let suite = "rove-mixed-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let workspace = Workspace(executable: engine, preferences: AppPreferences(defaults: defaults))
    let matrix = parent.appendingPathComponent("combined.mixed.json")
    try await workspace.combineInput(
      MixedInputRequest(
        morphology: root.appendingPathComponent("fixtures/validation/mixed/morphology.nex"),
        dna: root.appendingPathComponent("fixtures/validation/dna/complete.fasta"), output: matrix,
        missingTaxa: "reject"))
    workspace.open(matrix)
    try await wait(workspace)
    #expect(workspace.error == nil)
    #expect(workspace.report?.isMixed == true && workspace.settings.dataType == "mixed")
    #expect(workspace.settings.mixedRateError != nil)
    workspace.settings.morphologyRateMode = "estimate"
    workspace.settings.morphologyRateLogSD = "0,8"
    #expect(workspace.issue == nil)
    let report = try #require(workspace.report)
    #expect(report.mixed?.sources.map(\.dataType) == ["standard", "dna"])
    let draft = AnalysisDraft(
      matrix: matrix, report: report, settings: workspace.settings, outgroup: "Epsilon five")
    #expect(draft.formatVersion == 12)
    let file = parent.appendingPathComponent("mixed.rove.json")
    try draft.write(to: file)
    workspace.loadSetup(file)
    try await wait(workspace)
    #expect(workspace.error == nil && workspace.report?.mixed == draft.mixedInput)
    #expect(workspace.settings.morphologyLogSDValue == 0.8 && workspace.outgroup == "Epsilon five")
    var json = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any])
    json["formatVersion"] = 11
    try JSONSerialization.data(withJSONObject: json).write(to: file)
    #expect(throws: (any Error).self) { try AnalysisDraft.read(file) }
    workspace.settings.morphologyRateLogSD = "0"
    #expect(workspace.settings.validationError != nil)
    workspace.settings.morphologyRateMode = "fixed"
    workspace.settings.weights = [MoveOverride(move: "morphology_rate_scale", value: 1)]
    #expect(workspace.settings.validationError != nil)
  }

  @Test @MainActor func mixedRunsResumePredictEstimateImportAndExport() async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-mixed-run-\(UUID())")
    try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: parent) }
    let matrix = parent.appendingPathComponent("combined.mixed.json")
    try await MixedInputRequest(
      morphology: root.appendingPathComponent("fixtures/validation/mixed/morphology.nex"),
      dna: root.appendingPathComponent("fixtures/validation/dna/complete.fasta"), output: matrix,
      missingTaxa: "reject"
    ).execute(executable: engine)
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    for mode in ["fixed", "estimate", "dated"] {
      var settings = AnalysisSettings()
      settings.dataType = "mixed"
      settings.morphologyRateMode = mode == "fixed" ? "fixed" : "estimate"
      settings.morphologyRate = "2.5"
      settings.morphologyRateLogSD = "0.7"
      settings.clock = mode == "fixed" ? "strict" : "reference"
      settings.chains = mode == "fixed" ? 1 : 2
      settings.steps = 2000
      settings.sampleEvery = 100
      if mode == "estimate" {
        settings.treePrior = "yule"
        settings.birthRateUpper = "4"
      }
      if mode == "dated" {
        settings.dating.enabled = true
        settings.dating.birthRate = "0.8"
        settings.dating.deathRate = "0.3"
        settings.dating.fossilSamplingRate = "0.2"
        settings.dating.presentSampling = "0.7"
        settings.dating.origin = "4"
        settings.dating.clockRate = "0.1"
        settings.dating.clockPriorMean = "0.1"
        settings.dating.ages = Dictionary(
          uniqueKeysWithValues: report.allTaxa.map { ($0, TipAgeInput()) })
        settings.dating.ages["Beta two"] = TipAgeInput(kind: .fixed, fixed: "0.4")
      }
      let draft = AnalysisDraft(
        matrix: matrix, report: report, settings: settings, outgroup: "Epsilon five")
      let directory = parent.appendingPathComponent(mode)
      let worker = RunProcess()
      let paused = await worker.execute(
        RunRequest(id: UUID(), executable: engine, directory: directory, draft: draft)
      ) { event in
        if event.event == "started" { worker.pause() }
      }
      #expect(paused.status == .paused)
      #expect(
        await RunProcess().execute(
          RunRequest(
            id: UUID(), executable: engine, directory: directory, draft: draft, resume: true)
        ) { _ in }.status == .completed)
      let record = try EngineRecord.verify(directory: directory, draft: draft)
      #expect(record.model.mixedModel?.matches(settings) == true)
      let entry = HistoryEntry.read(directory)
      #expect(entry.draft?.mixedInput == report.mixed)
      let imported = try await CommandLineImport.prepare(
        directory.appendingPathComponent("results"), matrix: matrix, under: parent,
        executable: engine)
      #expect(
        HistoryEntry.read(imported).draft?.settings.morphologyRateMode
          == settings.morphologyRateMode)
      let prediction = PredictiveCheckModel(entry: entry, executable: engine)
      prediction.replicates = "2"
      prediction.start()
      await prediction.waitForExit()
      #expect(prediction.error == nil, Comment(rawValue: prediction.details ?? ""))
      let saved = try #require(prediction.predictions.first)
      let replicate = try await MatrixInspector.inspect(
        saved.directory.appendingPathComponent("matrices/replicate-00001.mixed.json"),
        gapsAsMissing: false, executable: engine)
      #expect(replicate.isMixed && replicate.partitions?.last?.states == 4)
      let marginal = MarginalLikelihoodModel(entry: entry, executable: engine)
      marginal.powers = "4"
      marginal.steps = "400"
      marginal.start()
      await marginal.waitForExit()
      #expect(marginal.error == nil, Comment(rawValue: marginal.details ?? ""))
      #expect(try #require(marginal.estimates.first).matches(draft))
      var changed = settings
      changed.morphologyRateLogSD = "0.9"
      changed.morphologyRate = "3"
      #expect(!record.model.dataModelMatches(changed))
      for include in [false, true] {
        let output = parent.appendingPathComponent("export-\(mode)-\(include)")
        _ = try await AssessmentRequest(
          entry: entry, executable: engine, output: output,
          options: AssessmentOptions(includeMatrix: include), export: true
        ).execute(with: EngineTool())
        #expect(
          FileManager.default.fileExists(
            atPath: output.appendingPathComponent("matrix.mixed.json").path) == include)
        if include {
          #expect(
            try Data(contentsOf: output.appendingPathComponent("matrix.mixed.json"))
              == Data(contentsOf: matrix))
        }
      }
    }
  }
}
