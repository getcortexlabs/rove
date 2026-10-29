import Foundation
import Testing

@testable import RoveApp

struct ParallelRunTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }
  private func request(_ directory: URL, steps: Int = 2_000) async throws -> RunRequest {
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = steps
    settings.sampleEvery = 100
    settings.chains = 2
    return RunRequest(
      id: UUID(), executable: engine, directory: directory,
      draft: AnalysisDraft(matrix: matrix, report: report, settings: settings, outgroup: "T20"))
  }

  @Test func groupedRunsReopenAndAnalyzeTheSelectedChain() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-parallel-\(UUID())")
    defer { try? FileManager.default.removeItem(at: directory) }
    let request = try await request(directory)
    let result = await RunProcess().execute(request) { _ in }
    #expect(result.status == .completed, "\(String(describing: result))")
    #expect(HistoryEntry.read(directory).status == .completed)
    let saved = try AnalysisDraft.read(directory.appendingPathComponent("setup.rove.json"))
    #expect(saved.settings.chains == 2)
    let report = try ChainComparison.read(request.output.appendingPathComponent("comparison.json"))
    #expect(report.chainCount == 2)
    #expect(report.retainedPerChain == 19)
    #expect(report.clades.contains { $0.constrained && $0.taxa.count == 19 })
    let chain = request.output.appendingPathComponent("chain-02")
    let trace = try await TraceDiagnosticsProcess().execute(
      directory: chain, executable: engine, burnin: 10)
    #expect(trace.report.source.path.hasSuffix("chain-02/trace.log"))
    let summary = try await TreeSummaryProcess().execute(
      directory: chain, executable: engine, burnin: 10, heights: "ca")
    #expect(
      FileManager.default.fileExists(
        atPath: summary.directory.appendingPathComponent("mcc.tree").path))
    try FileManager.default.removeItem(at: chain.appendingPathComponent("trace.log"))
    #expect(HistoryEntry.read(directory).status == .incomplete)
  }

  @Test @MainActor func cancellingAGroupClosesEveryWorkerAndKeepsHistory() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-parallel-cancel-\(UUID())")
    defer { try? FileManager.default.removeItem(at: directory) }
    let request = try await request(directory, steps: 100_000_000)
    let run = AnalysisRun(request: request)
    run.start()
    for _ in 0..<200 {
      if run.chainProgress.count == 2 || !run.isActive { break }
      try await Task.sleep(for: .milliseconds(20))
    }
    #expect(run.chainProgress.count == 2)
    run.cancel()
    await run.waitForExit()
    #expect(run.status == .cancelled)
    #expect(HistoryEntry.read(directory).status == .cancelled)
    for chain in 1...2 {
      let path = request.output.appendingPathComponent(String(format: "chain-%02d/run.json", chain))
      let record = try EngineRecord.read(path)
      #expect(record.status == "cancelled")
    }
  }

  @Test func anEarlyDiagnosticStopIsVerifiedAsACompletedGroup() async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-early-stop-\(UUID())")
    try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: parent) }
    let matrix = parent.appendingPathComponent("matrix.nex")
    try """
    #NEXUS
    Begin data;
    Dimensions ntax=5 nchar=6;
    Format symbols="01" missing=? gap=-;
    Matrix
    A 010110
    B 011010
    C 101-01
    D 10?101
    E_e 110011
    ;
    End;
    """.write(to: matrix, atomically: true, encoding: .utf8)
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: true, executable: engine)
    var settings = AnalysisSettings()
    settings.chains = 2
    settings.stopWhenConverged = true
    settings.clock = "strict"
    settings.gapsAsMissing = true
    settings.steps = 500_000
    settings.sampleEvery = 100
    let wrapper = parent.appendingPathComponent("engine")
    let quotedEngine = "'" + engine.path.replacingOccurrences(of: "'", with: "'\\''") + "'"
    // Loose diagnostic thresholds exercise an actual early engine finish in a
    // short test. Production UI requests retain the engine's 200 / 1.01 defaults.
    let script =
      "#!/bin/sh\nexec \(quotedEngine) \"$@\" --min-ess 1 --max-rhat 1000 --check-every 120000\n"
    try script.write(to: wrapper, atomically: true, encoding: .utf8)
    try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: wrapper.path)
    let request = RunRequest(
      id: UUID(), executable: wrapper, directory: parent.appendingPathComponent("session"),
      draft: AnalysisDraft(matrix: matrix, report: report, settings: settings))
    let result = await RunProcess().execute(request) { _ in }
    #expect(result.status == .completed, "\(String(describing: result))")
    let record = try EngineRecord.read(request.output.appendingPathComponent("run.json"))
    #expect(record.terminationReason == "diagnostic_thresholds")
    #expect(record.completedSteps < UInt64(settings.steps))
    #expect(HistoryEntry.read(request.directory).status == .completed)
  }

  @Test func legacySetupsRemainSingleChainAndInvalidCountsAreRejected() throws {
    let data = try JSONEncoder().encode(AnalysisSettings())
    var object = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
    object.removeValue(forKey: "chains")
    object.removeValue(forKey: "stopWhenConverged")
    let decoded = try JSONDecoder().decode(
      AnalysisSettings.self, from: JSONSerialization.data(withJSONObject: object))
    #expect(decoded.chains == 1 && !decoded.stopWhenConverged)
    var settings = decoded
    settings.stopWhenConverged = true
    #expect(settings.validationError != nil)
    settings.chains = 4
    #expect(settings.validationError == nil)
    settings.chains = 0
    #expect(settings.validationError != nil)
  }

  @Test func multiplexedEventsRejectMissingMembersAndStalePerChainSteps() throws {
    func data(_ event: String, chain: Int? = nil, step: Int) -> Data {
      var value: [String: Any] = [
        "schema_version": 1, "event": event, "step": step,
        "total_steps": 100, "elapsed_seconds": Double(step), "input_sha256": "hash",
        "posterior": -2, "likelihood": -1, "prior": -1, "termination_reason": "step_limit",
      ]
      if let chain { value["chain"] = chain } else { value["chain_count"] = 2 }
      return (try! JSONSerialization.data(withJSONObject: value)) + Data([10])
    }
    var reader = RunEventReader(steps: 100, fingerprint: "hash", chains: 2)
    #expect(throws: RunFailure.self) { try reader.append(data("started", chain: Int.min, step: 0)) }
    _ = try reader.append(data("started", chain: 2, step: 0))
    _ = try reader.append(data("progress", chain: 2, step: 50))
    _ = try reader.append(data("started", chain: 1, step: 0))
    #expect(throws: RunFailure.self) { try reader.append(data("group_completed", step: 100)) }
    #expect(throws: RunFailure.self) { try reader.append(data("progress", chain: 2, step: 20)) }
    _ = try reader.append(data("completed", chain: 1, step: 100))
    _ = try reader.append(data("completed", chain: 2, step: 100))
    _ = try reader.append(data("group_completed", step: 100))
    try reader.finish()
  }
}
