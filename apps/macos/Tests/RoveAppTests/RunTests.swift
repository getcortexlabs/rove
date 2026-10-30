import AppKit
import Darwin
import Foundation
import Testing

@testable import RoveApp

struct RunTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var executable: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }
  private func temporary() throws -> URL {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent("rove-test-\(UUID())")
    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: false)
    return url
  }
  private func request(
    in directory: URL, steps: Int = 1_000, engine: URL? = nil, multistate: Bool = false
  ) async throws
    -> RunRequest
  {
    let matrix = root.appendingPathComponent(
      multistate
        ? "fixtures/validation/multistate/matrix.nex" : "fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, executable: executable)
    var settings = AnalysisSettings()
    settings.steps = steps
    settings.sampleEvery = 100
    return RunRequest(
      id: UUID(), executable: engine ?? executable,
      directory: directory.appendingPathComponent("session"),
      draft: AnalysisDraft(
        matrix: matrix, report: report, settings: settings,
        monophyly: multistate ? [] : [Array(report.allTaxa.prefix(19))]))
  }

  @Test(arguments: [false, true]) func appAndDirectCLIPreserveScientificSamples(multistate: Bool)
    async throws
  {
    let directory = try temporary()
    defer { try? FileManager.default.removeItem(at: directory) }
    let request = try await request(in: directory, multistate: multistate)
    let result = await RunProcess().execute(request) { _ in }
    #expect(result.status == .completed, "\(String(describing: result))")
    let direct = directory.appendingPathComponent("direct")
    let child = Process()
    child.executableURL = executable
    child.arguments = request.draft.settings.arguments(
      matrix: URL(fileURLWithPath: request.draft.matrixPath), output: direct,
      monophyly: request.draft.monophyly)
    child.standardOutput = FileHandle.nullDevice
    child.standardError = FileHandle.nullDevice
    try child.run()
    child.waitUntilExit()
    #expect(child.terminationStatus == 0)
    for file in ["trace.log", "trees.nexus"] {
      func samples(_ folder: URL) throws -> [Substring] {
        try String(contentsOf: folder.appendingPathComponent(file), encoding: .utf8)
          .split(separator: "\n").filter { !$0.contains("command ") }
      }
      #expect(try samples(request.output) == samples(direct))
    }
    let session = try String(
      contentsOf: request.directory.appendingPathComponent("session.json"), encoding: .utf8)
    #expect(session.contains("\"completed\""))
    let draft = try AnalysisDraft.read(request.directory.appendingPathComponent("setup.rove.json"))
    #expect(draft.monophyly == request.draft.monophyly)
    let record = try EngineRecord.verify(directory: request.directory, draft: draft)
    #expect(record.model.partitions?.map(\.states) == (multistate ? [2, 3, 4] : [2]))
  }

  @Test @MainActor func liveProgressCancellationAndClosingReapTheEngine() async throws {
    let directory = try temporary()
    defer { try? FileManager.default.removeItem(at: directory) }
    let request = try await request(in: directory, steps: 100_000_000)
    let workspace = Workspace(executable: executable)
    let run = AnalysisRun(request: request)
    workspace.run = run
    run.start()
    for _ in 0..<100 {
      if run.progress != nil || !run.isActive { break }
      try await Task.sleep(for: .milliseconds(50))
    }
    #expect(run.progress != nil, "Progress must reach the main actor while the engine is running.")
    #expect(run.isActive)
    let progress = run.progress?.step
    run.receive(event("progress", step: 99), runID: UUID())
    #expect(run.progress?.step == progress)
    #expect(!workspace.canOpen(.model))
    _ = NSApplication.shared
    let window = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 400, height: 300),
      styleMask: [.titled, .closable], backing: .buffered, defer: false)
    window.isReleasedWhenClosed = false
    let keepRunning = WindowSetup.Coordinator(workspace: workspace, confirm: { false })
    #expect(!keepRunning.windowShouldClose(window))
    #expect(run.isActive)
    let close = WindowSetup.Coordinator(workspace: workspace, confirm: { true })
    window.delegate = close
    #expect(!close.windowShouldClose(window))
    // The close request returns immediately; its task stops and reaps the child.
    await run.waitForExit()
    window.close()
    #expect(run.status == .cancelled, "\(String(describing: run.result))")
    #expect(!workspace.isRunning)
    let final = run.progress?.step
    run.receive(event("progress", step: 99), runID: run.id)
    #expect(run.progress?.step == final)
    let record = try String(
      contentsOf: request.output.appendingPathComponent("run.json"), encoding: .utf8)
    #expect(record.contains("\"cancelled\""))
    let trees = try String(
      contentsOf: request.output.appendingPathComponent("trees.nexus"), encoding: .utf8)
    #expect(trees.trimmingCharacters(in: .whitespacesAndNewlines).hasSuffix("End;"))
    workspace.dismissRun()
    #expect(workspace.run == nil)
  }

  @Test func malformedEventsAndUnexpectedExitAreFailures() async throws {
    for body in [
      "printf 'not json\\n'\nexec /bin/sleep 30\n",
      "trap '' TERM\nprintf 'not json\\n'\nexec /bin/sleep 30\n", "exit 7\n",
      "printf '{\"schema_version\":1,\"event\":\"failed\",\"message\":\"fixture failure\"}\\n'\nexit 2\n",
    ] {
      let directory = try temporary()
      defer { try? FileManager.default.removeItem(at: directory) }
      let script = directory.appendingPathComponent("engine")
      try ("#!/bin/sh\nprintf '%s' \"$$\" > \"$0.pid\"\n" + body).write(
        to: script, atomically: true, encoding: .utf8)
      try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: script.path)
      let request = try await request(in: directory, engine: script)
      let began = Date()
      let result = await RunProcess().execute(request) { _ in }
      #expect(result.status == .failed)
      let pidText = try String(
        contentsOf: URL(fileURLWithPath: script.path + ".pid"), encoding: .utf8)
      let pid = try #require(Int32(pidText))
      #expect(Darwin.kill(pid, 0) == -1 && errno == ESRCH, "Failed engine must be reaped.")
      #expect(Date().timeIntervalSince(began) < 10, "Malformed engines must be stopped and reaped.")
      if body.contains("fixture failure") { #expect(result.details == "fixture failure") }
    }
  }

  @Test func launchFailureChangedInputAndOutputCollisionsNeverComplete() async throws {
    let directory = try temporary()
    defer { try? FileManager.default.removeItem(at: directory) }
    let request = try await request(
      in: directory, engine: directory.appendingPathComponent("missing"))
    let worker = RunProcess()
    #expect(await worker.execute(request) { _ in }.status == .failed)
    let record = request.directory.appendingPathComponent("session.json")
    try "existing record".write(to: record, atomically: true, encoding: .utf8)
    #expect(await RunProcess().execute(request) { _ in }.status == .failed)
    #expect(try String(contentsOf: record, encoding: .utf8) == "existing record")
    let matrix = directory.appendingPathComponent("changed.nex")
    try Data(contentsOf: URL(fileURLWithPath: request.draft.matrixPath)).write(to: matrix)
    let report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, executable: executable)
    let draft = AnalysisDraft(matrix: matrix, report: report, settings: request.draft.settings)
    try "\n".write(to: matrix, atomically: true, encoding: .utf8)
    let changed = RunRequest(
      id: UUID(), executable: executable,
      directory: directory.appendingPathComponent("changed"), draft: draft)
    #expect(
      await RunProcess().execute(changed) { _ in }.error == RunFailure.changedInput.errorDescription
    )
    #expect(!FileManager.default.fileExists(atPath: changed.output.path))
    #expect(
      try SessionRecord.read(changed.directory.appendingPathComponent("session.json")).status
        == "failed")
  }

  @Test @MainActor func reopeningChecksInputsAndPreservesExplicitGroups() async throws {
    let directory = try temporary()
    defer { try? FileManager.default.removeItem(at: directory) }
    let request = try await request(in: directory)
    let matrix = directory.appendingPathComponent("matrix.nex")
    try Data(contentsOf: URL(fileURLWithPath: request.draft.matrixPath)).write(to: matrix)
    let report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, executable: executable)
    let draft = AnalysisDraft(
      matrix: matrix, report: report, settings: request.draft.settings,
      monophyly: request.draft.monophyly)
    let setup = directory.appendingPathComponent("setup.rove.json")
    try draft.write(to: setup)
    let workspace = Workspace(executable: executable)
    func reopen() async throws {
      workspace.loadSetup(setup)
      for _ in 0..<100 {
        if !workspace.loading { return }
        try await Task.sleep(for: .milliseconds(50))
      }
      Issue.record("Setup did not reopen within five seconds.")
    }
    try await reopen()
    #expect(workspace.error == nil)
    #expect(workspace.monophyly == draft.monophyly)
    #expect(workspace.settings == draft.settings)
    #expect(workspace.run == nil)
    #expect(workspace.canStartRun)
    // Even a harmless byte change invalidates the saved fingerprint.
    var bytes = try Data(contentsOf: matrix)
    bytes.append(contentsOf: "\n".utf8)
    try bytes.write(to: matrix)
    try await reopen()
    #expect(workspace.error == RunFailure.changedInput.errorDescription)
    #expect(workspace.report?.inputSha256 == draft.matrixSHA256)
    try FileManager.default.removeItem(at: matrix)
    try await reopen()
    #expect(workspace.error?.contains("missing") == true)
    var json = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: setup)) as? [String: Any])
    json["formatVersion"] = 99
    try JSONSerialization.data(withJSONObject: json).write(to: setup)
    #expect(throws: (any Error).self) { try AnalysisDraft.read(setup) }
  }

  @Test func groupsMustBeKnownDistinctAndCompatible() {
    let taxa = ["A", "B", "C", "D"]
    #expect(
      GroupConstraints.validate([["A", "B"], ["A", "B", "C"], ["A", "B", "C", "D"]], taxa: taxa)
        == nil)
    #expect(GroupConstraints.validate([["A", "B"], ["C", "D"]], taxa: taxa) == nil)
    for groups in [
      [["A"]], [["A", "A"]], [["A", "X"]], [["A", "B"], ["B", "C"]], [["A", "B"], ["B", "A"]],
    ] {
      #expect(GroupConstraints.validate(groups, taxa: taxa) != nil)
    }
  }

  private func event(_ name: String, step: UInt64) -> RunEvent {
    RunEvent(
      schemaVersion: 1, event: name, step: step, totalSteps: 100,
      elapsedSeconds: Double(step), inputSha256: "hash", posterior: -2,
      likelihood: -1, prior: -1, message: nil)
  }

  @Test func progressProtocolRejectsStaleTruncatedAndUnsupportedEvents() throws {
    func bytes(_ name: String, step: Int, version: Int = 1) -> Data {
      Data(
        "{\"schema_version\":\(version),\"event\":\"\(name)\",\"step\":\(step),\"total_steps\":100,\"elapsed_seconds\":\(step),\"input_sha256\":\"hash\",\"posterior\":-2,\"likelihood\":-1,\"prior\":-1}\n"
          .utf8)
    }
    let started = bytes("started", step: 0)
    var reader = RunEventReader(steps: 100, fingerprint: "hash")
    #expect(try reader.append(started.prefix(20)).isEmpty)
    #expect(try reader.append(started.dropFirst(20)).count == 1)
    _ = try reader.append(bytes("progress", step: 50))
    #expect(throws: RunFailure.self) { try reader.append(bytes("progress", step: 40)) }
    for invalid in [
      bytes("started", step: 0, version: 2), bytes("progress", step: 10),
      bytes("completed", step: 100), Data(repeating: 65, count: 65_537),
    ] {
      var invalidReader = RunEventReader(steps: 100, fingerprint: "hash")
      #expect(throws: RunFailure.self) { try invalidReader.append(invalid) }
    }
    var truncated = RunEventReader(steps: 100, fingerprint: "hash")
    _ = try truncated.append(started.dropLast())
    #expect(throws: RunFailure.self) { try truncated.finish() }
    _ = try reader.append(bytes("completed", step: 100))
    try reader.finish()
    #expect(throws: RunFailure.self) { try reader.append(bytes("completed", step: 100)) }
  }
}
