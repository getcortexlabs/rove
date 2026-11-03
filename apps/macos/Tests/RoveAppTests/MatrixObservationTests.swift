import Foundation
import Testing

@testable import RoveApp

struct MatrixObservationTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL { root.appendingPathComponent("target/release/rove") }

  @Test @MainActor func labeledAlphabetsInspectRunAndPersistWithoutChangingInput() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-observations-\(UUID())")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let matrix = root.appendingPathComponent("fixtures/synthetic/exporter_labels.nex")
    let original = try Data(contentsOf: matrix)
    let observed = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, executable: engine)
    #expect(observed.constantCharacters == [1])
    #expect(observed.unobservedCharacters == [4])
    #expect(observed.uncertainCount == 1 && observed.polymorphicCount == 0)
    #expect(observed.observationExamples?.first?.taxon == "A")
    #expect(observed.observationExamples?.first?.character == 3)
    #expect(observed.samplerError != nil)
    let report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, partitioning: "labels", executable: engine)
    #expect(report.samplerError == nil && report.variableError != nil)
    #expect(report.partitions?.map(\.states) == [2, 3])
    #expect(report.stateLabels?[1] == ["small", "medium", "large"])
    var settings = AnalysisSettings()
    settings.partitioning = "labels"
    settings.conditioning = "none"
    settings.steps = 100
    settings.sampleEvery = 10
    let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings)
    #expect(draft.formatVersion == 8)
    let setup = directory.appendingPathComponent("saved.rove.json")
    try draft.write(to: setup)
    let restored = try AnalysisDraft.read(setup)
    #expect(restored.settings == settings)
    let request = RunRequest(
      id: UUID(), executable: engine, directory: directory.appendingPathComponent("run"),
      draft: restored)
    let run = AnalysisRun(request: request)
    run.start()
    await run.waitForExit()
    #expect(run.status == .completed, "\(run.result?.error ?? "unknown error")")
    let entry = HistoryEntry.read(request.directory)
    #expect(entry.draft?.settings.partitioning == "labels")
    #expect(entry.status == .completed)
    #expect(try Data(contentsOf: matrix) == original)
    var json = try #require(
      JSONSerialization.jsonObject(with: Data(contentsOf: setup)) as? [String: Any])
    json["formatVersion"] = 7
    try JSONSerialization.data(withJSONObject: json).write(to: setup)
    #expect(throws: (any Error).self) { try AnalysisDraft.read(setup) }
  }

  @Test func truePolymorphismIsVisibleAndBlocksInference() async throws {
    let file = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-polymorphism-\(UUID()).nex")
    defer { try? FileManager.default.removeItem(at: file) }
    let source = try String(
      contentsOf: root.appendingPathComponent(
        "fixtures/synthetic/exporter_labels.nex"), encoding: .utf8)
    try source.replacingOccurrences(of: "{01}", with: "(01)").write(
      to: file, atomically: true, encoding: .utf8)
    let report = try await MatrixInspector.inspect(
      file, gapsAsMissing: false, partitioning: "labels", executable: engine)
    #expect(report.polymorphicCount == 1 && report.uncertainCount == 0)
    #expect(report.observationExamples?.first?.value == "(01)")
    #expect(report.samplerError?.contains("polymorphism") == true)
  }
}
