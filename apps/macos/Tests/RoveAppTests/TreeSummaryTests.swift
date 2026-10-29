import Foundation
import Testing

@testable import RoveApp

struct TreeSummaryTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }
  private func completedRun(_ folder: URL) async throws -> RunRequest {
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = 1_000
    settings.sampleEvery = 100
    let request = RunRequest(
      id: UUID(), executable: engine, directory: folder,
      draft: AnalysisDraft(
        matrix: matrix, report: report, settings: settings, monophyly: [], outgroup: "T20"))
    #expect(await RunProcess().execute(request) { _ in }.status == .completed)
    return request
  }

  @Test @MainActor func summariesPersistWithoutChangingSamplesOrSetup() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-summary-app-\(UUID())")
    defer { try? FileManager.default.removeItem(at: directory) }
    let request = try await completedRun(directory)
    let files = [
      "setup.rove.json", "session.json", "results/trees.nexus", "results/run.json",
      "results/trace.log",
    ]
    let originals = try files.map { try Data(contentsOf: directory.appendingPathComponent($0)) }
    let model = TreeSummaryModel(directory: directory, executable: engine)
    model.burnin = "100"
    #expect(!model.valid)
    model.start()
    #expect(!model.busy)
    model.burnin = "25"
    model.start()
    await model.waitForExit()
    #expect(model.error == nil)
    let first = try #require(model.summaries.first)
    #expect(first.record.totalSamples == 11 && first.record.discardedSamples == 2)
    #expect(first.record.heightPolicy == "ca" && !first.record.convergenceAssessed)
    #expect(
      try Data(contentsOf: first.directory.appendingPathComponent("samples.trees")) == originals[2])
    let diagnostic = try await Task.detached {
      try await TraceDiagnosticsProcess().execute(
        directory: directory, executable: self.engine, burnin: 25)
    }.value
    #expect(diagnostic.report.source.completion == "completed")
    #expect(diagnostic.report.retainedSamples == first.record.retainedSamples)
    #expect(diagnostic.report.discardedSamples == first.record.discardedSamples)
    #expect(diagnostic.report.firstRetainedState == 200 && diagnostic.report.lastState == 1000)
    let annotated = try String(
      contentsOf: first.directory.appendingPathComponent("mcc.tree"), encoding: .utf8)
    #expect(annotated.contains("constrained=true") && annotated.contains("posterior="))
    model.heights = "keep"
    model.start()
    await model.waitForExit()
    #expect(model.error == nil && model.summaries.count == 2)
    let restored = TreeSummaryModel(directory: request.directory, executable: engine)
    await restored.load()
    await restored.load()
    #expect(restored.summaries.count == 2)
    #expect(Set(restored.summaries.map(\.id)) == Set(model.summaries.map(\.id)))
    for (file, original) in zip(files, originals) {
      #expect(try Data(contentsOf: directory.appendingPathComponent(file)) == original)
    }
    let explorer = TreeExplorer(
      directory: first.directory, runDirectory: directory, executable: engine)
    await explorer.load()
    #expect(explorer.error == nil && explorer.outgroup == "T20")
    let tree = try #require(explorer.document)
    #expect(tree.nodes.count == 39 && tree.summary.retainedSamples == 9)
    let tip = try #require(tree.nodes.first { $0.taxon == "T20" })
    explorer.selected = tree.root
    explorer.toggleCollapse()
    await explorer.waitForLayout()
    #expect(explorer.layout?.rows == 1)
    explorer.query = "T20"
    await explorer.search()
    #expect(explorer.matches == [tip.id])
    explorer.reveal(tip.id)
    await explorer.waitForLayout()
    #expect(explorer.collapsed.isEmpty && explorer.layout?.rows == 20)
    explorer.selected = tree.root
    explorer.rotate()
    await explorer.waitForLayout()
    #expect(explorer.document?.nodes.map(\.children) == tree.nodes.map(\.children))
    for (file, original) in zip(files, originals) {
      #expect(try Data(contentsOf: directory.appendingPathComponent(file)) == original)
    }
    try FileManager.default.removeItem(at: first.directory.appendingPathComponent("mcc.tree"))
    let reloaded = TreeSummaryModel(directory: request.directory, executable: engine)
    await reloaded.load()
    #expect(reloaded.summaries.count == 1)
  }

  @Test func cancellationAndIncompleteRunsDoNotProduceSummaries() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-summary-cancel-\(UUID())")
    defer { try? FileManager.default.removeItem(at: directory) }
    _ = try await completedRun(directory)
    let worker = TreeSummaryProcess()
    worker.cancel()
    await #expect(throws: CancellationError.self) {
      try await worker.execute(directory: directory, executable: engine, burnin: 10, heights: "ca")
    }
    let source = directory.appendingPathComponent("results/trees.nexus")
    var content = try String(contentsOf: source, encoding: .utf8)
    content = content.replacingOccurrences(of: "End;", with: "")
    try content.write(to: source, atomically: true, encoding: .utf8)
    await #expect(throws: (any Error).self) {
      try await TreeSummaryProcess().execute(
        directory: directory, executable: engine, burnin: 10, heights: "ca")
    }
    let children = try FileManager.default.contentsOfDirectory(
      atPath: directory.appendingPathComponent("summaries").path)
    #expect(children.isEmpty)
    try FileManager.default.removeItem(at: directory.appendingPathComponent("results/run.json"))
    await #expect(throws: (any Error).self) {
      try await TreeSummaryProcess().execute(
        directory: directory, executable: engine, burnin: 10, heights: "ca")
    }
  }

  @Test @MainActor func largeSummaryLoadsWhileAnAnalysisContinues() async throws {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-large-viewer-\(UUID())")
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: folder) }
    let engine = engine
    let summary = folder.appendingPathComponent("summary")
    try await Task.detached {
      func subtree(_ start: Int, _ count: Int) -> String {
        if count == 1 { return "\(start)[&rate=1]:1" }
        return "(\(subtree(start, count / 2)),\(subtree(start + count / 2, count / 2)))[&rate=1]:1"
      }
      let tree = String(subtree(1, 4096).dropLast("[&rate=1]:1".count))
      let translation = (1...4096).map { "\($0) T\($0)" }.joined(separator: ",")
      let input = folder.appendingPathComponent("input.trees")
      try
        "#NEXUS\nBegin trees;Translate \(translation);tree STATE_0 = [&R] \(tree);tree STATE_100 = [&R] \(tree);End;"
        .write(to: input, atomically: true, encoding: .utf8)
      let process = Process()
      process.executableURL = engine
      process.arguments = [
        "summarize", input.path, "--output", summary.path, "--burnin-percent", "0", "--heights",
        "ca",
      ]
      process.standardOutput = FileHandle.nullDevice
      process.standardError = FileHandle.nullDevice
      try process.run()
      process.waitUntilExit()
      #expect(process.terminationStatus == 0)
    }.value
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = 100_000_000
    settings.sampleEvery = 1_000_000
    let request = RunRequest(
      id: UUID(), executable: engine, directory: folder.appendingPathComponent("active"),
      draft: AnalysisDraft(
        matrix: matrix, report: report, settings: settings, monophyly: [], outgroup: nil))
    let suite = "rove-large-viewer-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let workspace = Workspace(
      executable: engine, preferences: AppPreferences(defaults: defaults),
      history: RunHistory(defaults: defaults, defaultRoot: folder))
    let run = AnalysisRun(request: request)
    workspace.run = run
    run.start()
    workspace.openTree(summary, run: folder)
    let explorer = try #require(workspace.treeExplorer)
    var finished = false
    let load = Task {
      await explorer.load()
      finished = true
    }
    var navigations = 0
    while !finished {
      workspace.openHistory()
      workspace.returnToWorkspace()
      navigations += 1
      try await Task.sleep(for: .milliseconds(5))
    }
    await load.value
    #expect(explorer.error == nil && explorer.document?.nodes.count == 8191)
    #expect(navigations > 1 && run.isActive && run.progress != nil)
    #expect(workspace.run?.id == run.id && !workspace.showingHistory)
    await workspace.cancelAndWait()
    #expect(run.status == .cancelled)
  }

}
