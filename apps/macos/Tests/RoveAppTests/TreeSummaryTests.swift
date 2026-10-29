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
    #expect(RunProcess().execute(request) { _ in }.status == .completed)
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
    #expect(throws: CancellationError.self) {
      try worker.execute(directory: directory, executable: engine, burnin: 10, heights: "ca")
    }
    let source = directory.appendingPathComponent("results/trees.nexus")
    var content = try String(contentsOf: source, encoding: .utf8)
    content = content.replacingOccurrences(of: "End;", with: "")
    try content.write(to: source, atomically: true, encoding: .utf8)
    #expect(throws: (any Error).self) {
      try TreeSummaryProcess().execute(
        directory: directory, executable: engine, burnin: 10, heights: "ca")
    }
    let children = try FileManager.default.contentsOfDirectory(
      atPath: directory.appendingPathComponent("summaries").path)
    #expect(children.isEmpty)
    try FileManager.default.removeItem(at: directory.appendingPathComponent("results/run.json"))
    #expect(throws: (any Error).self) {
      try TreeSummaryProcess().execute(
        directory: directory, executable: engine, burnin: 10, heights: "ca")
    }
  }
}
