import Foundation
import Testing

@testable import RoveApp

struct NucleotideTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }

  @MainActor private func waitForImport(_ workspace: Workspace) async throws {
    for _ in 0..<1000 {
      if !workspace.loading { return }
      try await Task.sleep(for: .milliseconds(10))
    }
    throw ImportError.message("DNA import did not finish")
  }

  @Test @MainActor func importChoosesJc69AndPersistsItsDataIdentity() async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-dna-import-\(UUID())")
    try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: parent) }
    let suite = "rove-dna-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let workspace = Workspace(executable: engine, preferences: AppPreferences(defaults: defaults))
    let matrix = root.appendingPathComponent("fixtures/validation/dna/matrix.fasta")
    workspace.load(matrix)
    try await waitForImport(workspace)
    #expect(workspace.error == nil)
    #expect(workspace.report?.dataType == "dna" && workspace.report?.inputFormat == "fasta")
    #expect(workspace.report?.allTaxa.first == "Alpha one")
    #expect(workspace.settings.dataType == "dna" && workspace.settings.conditioning == "none")
    #expect(workspace.settings.partitioning == "declared" && workspace.issue != nil)
    workspace.settings.gapsAsMissing = true
    workspace.reloadGapPolicy()
    try await waitForImport(workspace)
    #expect(workspace.issue == nil)
    let report = try #require(workspace.report)
    #expect(report.partitions?.first?.states == 4)
    let draft = AnalysisDraft(
      matrix: matrix, report: report, settings: workspace.settings, outgroup: "Epsilon five")
    #expect(draft.formatVersion == 11)
    let file = parent.appendingPathComponent("dna.rove.json")
    try draft.write(to: file)
    workspace.loadSetup(file)
    try await waitForImport(workspace)
    #expect(workspace.error == nil && workspace.settings.dataType == "dna")
    #expect(workspace.outgroup == "Epsilon five")
    var json = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any])
    json["formatVersion"] = 10
    try JSONSerialization.data(withJSONObject: json).write(to: file)
    #expect(throws: (any Error).self) { try AnalysisDraft.read(file) }
    workspace.settings.conditioning = "variable"
    #expect(workspace.settings.validationError != nil)
    workspace.settings.conditioning = "none"
    workspace.settings.partitioning = "observed"
    #expect(workspace.settings.validationError != nil)
    workspace.load(root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"))
    try await waitForImport(workspace)
    #expect(
      workspace.settings.dataType == "standard" && workspace.settings.conditioning == "variable")
  }

  @Test @MainActor func dnaRunsResumeSummarizePredictAndEstimate() async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-dna-run-\(UUID())")
    try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: parent) }
    for clock in ["strict", "reference"] {
      let ambiguous = clock == "reference"
      let matrix = root.appendingPathComponent(
        "fixtures/validation/dna/" + (ambiguous ? "matrix.nex" : "complete.fasta"))
      let report = try await MatrixInspector.inspect(
        matrix, gapsAsMissing: ambiguous, executable: engine)
      var settings = AnalysisSettings()
      settings.dataType = "dna"
      settings.conditioning = "none"
      settings.partitioning = "declared"
      settings.gapsAsMissing = ambiguous
      settings.clock = clock
      settings.steps = 2_000
      settings.sampleEvery = 100
      settings.chains = ambiguous ? 2 : 1
      if ambiguous {
        settings.treePrior = "yule"
        settings.birthRateUpper = "4"
      }
      let draft = AnalysisDraft(
        matrix: matrix, report: report, settings: settings, outgroup: "Epsilon five")
      let directory = parent.appendingPathComponent(clock)
      let worker = RunProcess()
      let request = RunRequest(id: UUID(), executable: engine, directory: directory, draft: draft)
      let paused = await worker.execute(request) { event in
        if event.event == "started" { worker.pause() }
      }
      #expect(paused.status == .paused)
      let resumed = RunRequest(
        id: UUID(), executable: engine, directory: directory, draft: draft, resume: true)
      #expect(await RunProcess().execute(resumed) { _ in }.status == .completed)
      let record = try EngineRecord.verify(directory: directory, draft: draft)
      #expect(record.model.dataType == "dna" && record.model.substitutionModel == "jc69")
      #expect(record.model.conditioningLabel == "JC69")
      let entry = HistoryEntry.read(directory)
      #expect(entry.draft?.settings.dataType == "dna")
      let imported = try await CommandLineImport.prepare(
        directory.appendingPathComponent("results"), matrix: matrix, under: parent,
        executable: engine)
      #expect(HistoryEntry.read(imported).draft?.formatVersion == 11)

      let summaryDirectory =
        ambiguous ? directory.appendingPathComponent("results/chain-01") : directory
      let summary = TreeSummaryModel(directory: summaryDirectory, executable: engine)
      summary.start()
      await summary.waitForExit()
      #expect(summary.error == nil, Comment(rawValue: summary.details ?? ""))
      #expect(summary.summaries.count == 1)

      let prediction = PredictiveCheckModel(entry: entry, executable: engine)
      prediction.replicates = "2"
      prediction.start()
      await prediction.waitForExit()
      if ambiguous {
        #expect(prediction.error != nil && prediction.predictions.isEmpty)
        #expect(prediction.details?.contains("observation model") == true)
      } else {
        #expect(prediction.error == nil, Comment(rawValue: prediction.details ?? ""))
        let saved = try #require(prediction.predictions.first)
        let replicate = try String(
          contentsOf: saved.directory.appendingPathComponent("matrices/replicate-00001.nex"),
          encoding: .utf8)
        #expect(replicate.contains("DATATYPE=DNA"))
        #expect(
          FileManager.default.fileExists(
            atPath: saved.directory.appendingPathComponent("source/matrix.fasta").path))
      }
      let marginal = MarginalLikelihoodModel(entry: entry, executable: engine)
      marginal.powers = "4"
      marginal.steps = "400"
      marginal.start()
      await marginal.waitForExit()
      #expect(marginal.error == nil, Comment(rawValue: marginal.details ?? ""))
      let saved = try #require(marginal.estimates.first)
      #expect(saved.matches(draft) && saved.record.model.substitutionModel == "jc69")
      var changed = settings
      changed.dataType = "standard"
      #expect(
        !record.matches(
          AnalysisDraft(matrix: matrix, report: report, settings: changed, outgroup: "Epsilon five")
        ))
    }
  }
}
