import Foundation
import Testing

@testable import RoveApp

struct HistoryTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }
  private func temporary() throws -> URL {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-history-\(UUID())")
    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: false)
    return url
  }
  private func request(_ directory: URL, steps: Int = 1_000) async throws -> RunRequest {
    let matrix = directory.deletingLastPathComponent().appendingPathComponent("matrix.nex")
    if !FileManager.default.fileExists(atPath: matrix.path) {
      try Data(contentsOf: root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"))
        .write(to: matrix)
    }
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = steps
    settings.sampleEvery = 100
    return RunRequest(
      id: UUID(), executable: engine, directory: directory,
      draft: AnalysisDraft(
        matrix: matrix, report: report, settings: settings,
        monophyly: [Array(report.allTaxa.prefix(19))]))
  }

  @Test func completedCancelledAndFailedRunsHaveReadableHistory() async throws {
    let directory = try temporary()
    defer { try? FileManager.default.removeItem(at: directory) }
    let completed = try await request(directory.appendingPathComponent("completed"))
    #expect(await RunProcess().execute(completed) { _ in }.status == .completed)
    let first = HistoryEntry.read(completed.directory)
    #expect(first.status == .completed)
    #expect(first.completedSteps == 1_000)
    #expect(first.elapsedSeconds != nil && first.startedAt != nil)
    #expect(first.draft?.monophyly == completed.draft.monophyly)
    #expect(
      first.files.contains("results/trace.log") && first.files.contains("results/trees.nexus"))

    let cancelled = try await request(
      directory.appendingPathComponent("cancelled"), steps: 100_000_000)
    let worker = RunProcess()
    let result = await worker.execute(cancelled) { event in
      if event.event == "started" { worker.cancel() }
    }
    #expect(result.status == .cancelled)
    let second = HistoryEntry.read(cancelled.directory)
    #expect(second.status == .cancelled)
    #expect(try #require(second.completedSteps) < 100_000_000)
    #expect(second.elapsedSeconds != nil)

    let failed = RunRequest(
      id: UUID(), executable: directory.appendingPathComponent("missing-engine"),
      directory: directory.appendingPathComponent("failed"), draft: completed.draft)
    #expect(await RunProcess().execute(failed) { _ in }.status == .failed)
    let third = HistoryEntry.read(failed.directory)
    #expect(third.status == .failed)
    #expect(third.message != nil)
    #expect(third.elapsedSeconds != nil)
    #expect(third.completedSteps == nil)

    let early = try await request(directory.appendingPathComponent("early-cancel"))
    let earlyWorker = RunProcess()
    earlyWorker.cancel()
    #expect(await earlyWorker.execute(early) { _ in }.status == .cancelled)
    let beforeLaunch = HistoryEntry.read(early.directory)
    #expect(beforeLaunch.status == .cancelled && beforeLaunch.completedSteps == 0)
    #expect(!FileManager.default.fileExists(atPath: early.output.path))

    let scan = HistoryScan.read(roots: [directory], known: [])
    #expect(scan.entries.count == 4)
    #expect(Set(scan.entries.map(\.status)) == [.completed, .cancelled, .failed])
  }

  @Test func legacyRecordsUseEngineTimingAndPartialRecordsStayInterrupted() async throws {
    let directory = try temporary()
    defer { try? FileManager.default.removeItem(at: directory) }
    let request = try await request(directory.appendingPathComponent("run"))
    #expect(await RunProcess().execute(request) { _ in }.status == .completed)
    // Older sessions have no checkpoint to confirm a lost completion event.
    let checkpoint = request.output.appendingPathComponent("checkpoint.json")
    if FileManager.default.fileExists(atPath: checkpoint.path) {
      try FileManager.default.removeItem(at: checkpoint)
    }
    let file = request.directory.appendingPathComponent("session.json")
    var json = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any])
    for key in ["startedAt", "finishedAt", "elapsedSeconds", "completedSteps"] {
      json.removeValue(forKey: key)
    }
    try JSONSerialization.data(withJSONObject: json).write(to: file)
    let legacy = HistoryEntry.read(request.directory)
    let engine = try EngineRecord.read(request.output.appendingPathComponent("run.json"))
    #expect(legacy.status == .completed)
    #expect(legacy.elapsedSeconds == engine.wallSeconds)
    #expect(legacy.completedSteps == engine.completedSteps)
    #expect(legacy.startedAt == engine.startedUnixSeconds.map { Date(timeIntervalSince1970: $0) })
    json["status"] = "partial"
    json.removeValue(forKey: "exitCode")
    try JSONSerialization.data(withJSONObject: json).write(to: file)
    #expect(HistoryEntry.read(request.directory).status == .interrupted)
    #expect(HistoryEntry.read(request.directory).completedSteps == 1_000)
    try FileManager.default.removeItem(at: request.output.appendingPathComponent("run.json"))
    let interrupted = HistoryEntry.read(request.directory)
    #expect(interrupted.status == .interrupted)
    #expect(interrupted.elapsedSeconds == nil && interrupted.completedSteps == nil)
  }

  @Test func missingDamagedAndMismatchedRecordsNeverLookCompleted() async throws {
    let directory = try temporary()
    defer { try? FileManager.default.removeItem(at: directory) }
    let request = try await request(directory.appendingPathComponent("run"))
    #expect(await RunProcess().execute(request) { _ in }.status == .completed)
    let engineFile = request.output.appendingPathComponent("run.json")
    let original = try Data(contentsOf: engineFile)
    var json = try #require(JSONSerialization.jsonObject(with: original) as? [String: Any])
    var input = try #require(json["input"] as? [String: Any])
    input["sha256"] = String(repeating: "0", count: 64)
    json["input"] = input
    try JSONSerialization.data(withJSONObject: json).write(to: engineFile)
    #expect(HistoryEntry.read(request.directory).status == .incomplete)
    json = try #require(JSONSerialization.jsonObject(with: original) as? [String: Any])
    var model = try #require(json["model"] as? [String: Any])
    model["preset"] = "strict"
    json["model"] = model
    try JSONSerialization.data(withJSONObject: json).write(to: engineFile)
    #expect(HistoryEntry.read(request.directory).status == .incomplete)
    try original.write(to: engineFile)
    try FileManager.default.removeItem(at: request.output.appendingPathComponent("trees.nexus"))
    let missing = HistoryEntry.read(request.directory)
    #expect(missing.status == .incomplete)
    #expect(!missing.files.contains("results/trees.nexus"))
    let record = request.directory.appendingPathComponent("session.json")
    var session = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: record)) as? [String: Any])
    session["formatVersion"] = 99
    try JSONSerialization.data(withJSONObject: session).write(to: record)
    #expect(HistoryEntry.read(request.directory).status == .unreadable)
    try Data(repeating: 65, count: 2 * 1024 * 1024 + 1).write(to: record)
    let corrupt = HistoryEntry.read(request.directory)
    #expect(corrupt.status == .unreadable)
    #expect(corrupt.draft?.matrixSHA256 == request.draft.matrixSHA256)
    try FileManager.default.removeItem(at: request.directory)
    #expect(HistoryEntry.read(request.directory).status == .unavailable)
  }

  @Test @MainActor func locationsAndDiscoveredRunsPersistWithoutDuplicates() async throws {
    let directory = try temporary()
    let suite = "rove-history-test-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer {
      defaults.removePersistentDomain(forName: suite)
      try? FileManager.default.removeItem(at: directory)
    }
    let defaultRoot = directory.appendingPathComponent("default")
    let customRoot = directory.appendingPathComponent("custom")
    for folder in [defaultRoot, customRoot] {
      try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false)
    }
    let first = try await request(defaultRoot.appendingPathComponent("first"))
    let second = try await request(customRoot.appendingPathComponent("second"))
    for request in [first, second] {
      #expect(await RunProcess().execute(request) { _ in }.status == .completed)
    }
    let history = RunHistory(defaults: defaults, defaultRoot: defaultRoot)
    history.rememberLocation(customRoot)
    let alias = directory.appendingPathComponent("alias")
    try FileManager.default.createSymbolicLink(at: alias, withDestinationURL: second.directory)
    history.rememberRun(alias)
    history.rememberRun(second.directory)
    await history.reload()
    #expect(history.entries.count == 2)
    let restored = RunHistory(defaults: defaults, defaultRoot: defaultRoot)
    await restored.reload()
    #expect(Set(restored.entries.map(\.id)) == Set(history.entries.map(\.id)))
    try FileManager.default.removeItem(at: second.directory)
    try FileManager.default.removeItem(at: customRoot)
    await restored.reload()
    #expect(restored.entries.count == 2)
    #expect(restored.entries.contains { $0.status == .unavailable })
    #expect(restored.unavailableRoots.contains(customRoot.path))
  }

  @Test @MainActor func browsingPreservesAnActiveRunAndReuseChecksTheMatrix() async throws {
    let directory = try temporary()
    let suite = "rove-history-workspace-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer {
      defaults.removePersistentDomain(forName: suite)
      try? FileManager.default.removeItem(at: directory)
    }
    let request = try await request(
      directory.appendingPathComponent("original"), steps: 100_000_000)
    // Save a failed launch so the history fixture is real without waiting for a long chain.
    let failed = RunRequest(
      id: request.id, executable: directory.appendingPathComponent("missing"),
      directory: request.directory, draft: request.draft)
    #expect(await RunProcess().execute(failed) { _ in }.status == .failed)
    let entry = HistoryEntry.read(request.directory)
    let history = RunHistory(defaults: defaults, defaultRoot: directory)
    let workspace = Workspace(
      executable: engine, preferences: AppPreferences(defaults: defaults), history: history)
    workspace.showingHistory = true
    workspace.reuseSetup(entry)
    try await finishImport(workspace)
    #expect(!workspace.showingHistory)
    #expect(workspace.settings == request.draft.settings)
    #expect(workspace.monophyly == request.draft.monophyly)
    #expect(workspace.canStartRun && workspace.run == nil)
    workspace.outputParent = directory
    workspace.startRun()
    let active = try #require(workspace.run)
    workspace.openHistory()
    #expect(workspace.showingHistory && workspace.isRunning)
    #expect(!workspace.canStartRun)
    let oldSettings = workspace.settings
    workspace.reuseSetup(entry)
    #expect(workspace.run?.id == active.id && workspace.settings == oldSettings)
    workspace.returnToWorkspace()
    #expect(workspace.run?.id == active.id && !workspace.showingHistory)
    await workspace.cancelAndWait()
    #expect(history.entries.contains { $0.status == .cancelled })
    let previousHash = workspace.report?.inputSha256
    let matrix = URL(fileURLWithPath: request.draft.matrixPath)
    var bytes = try Data(contentsOf: matrix)
    bytes.append(contentsOf: "\n".utf8)
    try bytes.write(to: matrix)
    workspace.showingHistory = true
    workspace.reuseSetup(entry)
    try await finishImport(workspace)
    #expect(workspace.error == RunFailure.changedInput.errorDescription)
    #expect(workspace.showingHistory && workspace.report?.inputSha256 == previousHash)
    try FileManager.default.removeItem(at: matrix)
    workspace.reuseSetup(entry)
    try await finishImport(workspace)
    #expect(workspace.error?.contains("missing") == true)
  }

  @MainActor private func finishImport(_ workspace: Workspace) async throws {
    for _ in 0..<100 {
      if !workspace.loading { return }
      try await Task.sleep(for: .milliseconds(50))
    }
    throw ImportError.message("Setup import did not finish.")
  }
}
