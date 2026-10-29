import Foundation
import Testing

@testable import RoveApp

struct CheckpointTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }

  @Test func pausedGroupsResumeFromHistoryWithoutChangingSamples() async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-resume-\(UUID())")
    defer { try? FileManager.default.removeItem(at: parent) }
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    for chains in [1, 4] {
      var settings = AnalysisSettings()
      settings.steps = 100_000
      settings.sampleEvery = 100
      settings.chains = chains
      let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings, outgroup: "T20")
      let directory = parent.appendingPathComponent("paused-\(chains)")
      let request = RunRequest(id: UUID(), executable: engine, directory: directory, draft: draft)
      let worker = RunProcess()
      let result = await worker.execute(request) { event in
        if event.event == "progress", (event.step ?? 0) >= 1_000 { worker.pause() }
      }
      #expect(result.status == .paused, "\(String(describing: result))")
      let entry = HistoryEntry.read(directory)
      #expect(entry.status == .paused)
      let checkpoint = try #require(entry.checkpoint)
      #expect(checkpoint.step >= 1_000 && checkpoint.step < 100_000)
      let resumed = RunRequest(
        id: UUID(), executable: engine, directory: directory, draft: draft, resume: true)
      let continued = await RunProcess().execute(resumed) { _ in }
      #expect(continued.status == .completed, "\(String(describing: continued))")
      #expect(HistoryEntry.read(directory).status == .completed)
      let fullDirectory = parent.appendingPathComponent("full-\(chains)")
      let full = RunRequest(id: UUID(), executable: engine, directory: fullDirectory, draft: draft)
      let complete = await RunProcess().execute(full) { _ in }
      #expect(complete.status == .completed)
      for chain in 1...chains {
        let component = chains == 1 ? "results" : String(format: "results/chain-%02d", chain)
        for file in ["trace.log", "trees.nexus"] {
          func samples(_ folder: URL) throws -> [Substring] {
            try String(
              contentsOf: folder.appendingPathComponent(component).appendingPathComponent(file),
              encoding: .utf8
            )
            .split(separator: "\n").filter {
              !$0.hasPrefix("# command") && !$0.hasPrefix("[command")
            }
          }
          #expect(try samples(directory) == samples(fullDirectory))
        }
      }
    }
  }

  @Test func interruptedSessionOffersRecoveryAndChangedInputKeepsItsRecord() async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-recovery-\(UUID())")
    try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: parent) }
    let matrix = parent.appendingPathComponent("matrix.nex")
    try FileManager.default.copyItem(
      at: root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"), to: matrix)
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = 100_000
    settings.sampleEvery = 100
    let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings)
    let directory = parent.appendingPathComponent("run")
    let request = RunRequest(id: UUID(), executable: engine, directory: directory, draft: draft)
    let worker = RunProcess()
    let result = await worker.execute(request) { event in
      if event.event == "started" { worker.pause() }
    }
    #expect(result.status == .paused)
    let record = directory.appendingPathComponent("session.json")
    var object = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: record)) as? [String: Any])
    object["status"] = "partial"
    let partial = try JSONSerialization.data(withJSONObject: object)
    try partial.write(to: record)
    let entry = HistoryEntry.read(directory)
    #expect(entry.status == .interrupted && entry.checkpoint != nil)
    let original = try Data(contentsOf: matrix)
    try (original + Data([10])).write(to: matrix)
    let resume = RunRequest(
      id: UUID(), executable: engine, directory: directory, draft: draft, resume: true)
    let refused = await RunProcess().execute(resume) { _ in }
    #expect(refused.status == .failed)
    #expect(try Data(contentsOf: record) == partial)
    try original.write(to: matrix)
    let recovered = await RunProcess().execute(resume) { _ in }
    #expect(recovered.status == .completed, "\(String(describing: recovered))")
    // Simulate losing the terminal event after the engine finished durably.
    try partial.write(to: record)
    let finished = HistoryEntry.read(directory)
    #expect(finished.status == .completed)
    #expect(finished.completedSteps == UInt64(settings.steps))
    #expect(finished.checkpoint == nil)
  }
}
