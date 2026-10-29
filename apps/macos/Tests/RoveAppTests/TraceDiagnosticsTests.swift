import Foundation
import Testing

@testable import RoveApp

struct TraceDiagnosticsTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }
  private func fixture() throws -> URL {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-trace-tests-\(UUID())")
    try FileManager.default.createDirectory(
      at: directory.appendingPathComponent("results"), withIntermediateDirectories: true)
    try FileManager.default.copyItem(
      at: root.appendingPathComponent("fixtures/diagnostics/synthetic.tsv"),
      to: directory.appendingPathComponent("results/trace.log"))
    return directory
  }

  @Test @MainActor func burninPersistsAndReopeningReadsTheCurrentTrace() async throws {
    let directory = try fixture()
    defer { try? FileManager.default.removeItem(at: directory) }
    let source = directory.appendingPathComponent("results/trace.log")
    let original = try Data(contentsOf: source)
    let model = TraceDiagnosticsModel(directory: directory, executable: engine)
    model.burnin = "100"
    #expect(!model.valid)
    model.start()
    #expect(!model.busy)
    model.burnin = "25"
    model.start()
    await model.waitForExit()
    #expect(model.error == nil)
    let first = try #require(model.result)
    #expect(first.report.discardedSamples == 512 && first.report.retainedSamples == 1537)
    #expect(first.report.firstRetainedState == 51200)
    #expect(first.report.source.completion == "unverified")
    let column = try #require(first.report.columns.first { $0.name == "ar_positive" })
    let plot = TracePlot(
      column: column, report: first.report, strings: AppStrings(language: .english),
      includeDiscarded: false)
    #expect(plot.points.allSatisfy { $0.retained })
    #expect(plot.x(first.report.firstRetainedState) == 0 && plot.x(first.report.lastState) == 1)
    let highest = try #require(plot.points.map(\.value).max())
    #expect(abs(plot.y(highest) - 1) < 1e-10)
    model.selected = "constant"
    #expect(model.statistic?.ess == nil && model.statistic?.essReason == "constant")
    model.query = "ar_"
    #expect(model.columns.count == 2)
    let reopened = TraceDiagnosticsModel(directory: directory, executable: engine)
    await reopened.load()
    #expect(
      reopened.burnin == "25" && reopened.result?.report.source.sha256 == first.report.source.sha256
    )
    #expect(reopened.result?.directory != first.directory)
    #expect(try Data(contentsOf: source) == original)
    try Data("Sample posterior\n0 NaN\n".utf8).write(to: source)
    reopened.start()
    await reopened.waitForExit()
    #expect(
      reopened.error != nil && reopened.result?.report.source.sha256 == first.report.source.sha256)
    #expect(try TraceReport.read(first.directory).retainedSamples == 1537)
  }

  @Test func invalidSavedPlotsAndNonfiniteTracesAreRejected() throws {
    let directory = try fixture()
    defer { try? FileManager.default.removeItem(at: directory) }
    let saved = try TraceDiagnosticsProcess().execute(
      directory: directory, executable: engine, burnin: 10)
    let path = saved.directory.appendingPathComponent("diagnostics.json")
    var json = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: path)) as? [String: Any])
    json["convergence_assessed"] = true
    try JSONSerialization.data(withJSONObject: json).write(to: path)
    #expect(throws: (any Error).self) { try TraceReport.read(saved.directory) }
    let worker = TraceDiagnosticsProcess()
    worker.cancel()
    #expect(throws: CancellationError.self) {
      try worker.execute(directory: directory, executable: engine, burnin: 10)
    }
  }

  @Test func ownerCancellationStopsARealCalculation() async throws {
    let directory = try fixture()
    defer { try? FileManager.default.removeItem(at: directory) }
    let trace = directory.appendingPathComponent("results/trace.log")
    var text = "Sample a b c d e f g h\n"
    for i in 0..<50_000 {
      text +=
        "\(i * 100) "
        + Array(repeating: String(Double(i) / 50_000), count: 8).joined(separator: " ") + "\n"
    }
    try text.write(to: trace, atomically: true, encoding: .utf8)
    let worker = TraceDiagnosticsProcess()
    let executable = engine
    let task = Task.detached {
      try worker.execute(directory: directory, executable: executable, burnin: 10)
    }
    try await Task.sleep(for: .milliseconds(20))
    worker.cancel()
    do {
      _ = try await task.value
      Issue.record("Cancellation should stop the diagnostic process")
    } catch is CancellationError {
    }
    let children = try FileManager.default.contentsOfDirectory(
      atPath: directory.appendingPathComponent("diagnostics").path)
    #expect(children.isEmpty)
    #expect(try String(contentsOf: trace, encoding: .utf8) == text)
  }

  @Test @MainActor func diagnosticsLeaveNavigationAndAnActiveAnalysisAvailable() async throws {
    let directory = try fixture()
    defer { try? FileManager.default.removeItem(at: directory) }
    let trace = directory.appendingPathComponent("results/trace.log")
    var text = "Sample posterior likelihood prior\n"
    for i in 0..<50_000 { text += "\(i * 100) \(i) \(i + 1) -1\n" }
    try text.write(to: trace, atomically: true, encoding: .utf8)
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = 100_000_000
    settings.sampleEvery = 1_000_000
    let request = RunRequest(
      id: UUID(), executable: engine, directory: directory.appendingPathComponent("active"),
      draft: AnalysisDraft(
        matrix: matrix, report: report, settings: settings, monophyly: [], outgroup: nil))
    let suite = "rove-diagnostics-navigation-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let workspace = Workspace(
      executable: engine, preferences: AppPreferences(defaults: defaults),
      history: RunHistory(defaults: defaults, defaultRoot: directory))
    let run = AnalysisRun(request: request)
    workspace.run = run
    run.start()
    defer { run.cancel() }
    workspace.openDiagnostics(in: directory)
    let model = try #require(workspace.traceDiagnostics)
    model.start()
    var ticks = 0
    while model.busy {
      workspace.openHistory()
      workspace.returnToWorkspace()
      ticks += 1
      try await Task.sleep(for: .milliseconds(5))
    }
    await model.waitForExit()
    #expect(ticks > 1 && model.error == nil && model.result?.report.totalSamples == 50_000)
    #expect(run.isActive && workspace.run?.id == run.id && !workspace.showingHistory)
    await workspace.cancelAndWait()
    #expect(run.status == .cancelled)
  }

}
