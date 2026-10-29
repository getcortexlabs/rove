import Foundation
import Testing

@testable import RoveApp

struct AnalysisTests {
  private var root: URL {
    let package = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent()
    return package.deletingLastPathComponent().deletingLastPathComponent()
  }

  private var executable: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }

  @Test func invalidSeedsAndSamplingIntervalsAreRejected() {
    var settings = AnalysisSettings()
    for seed in ["-1", "1.5", "", "18446744073709551616", "$(whoami)"] {
      settings.seed = seed
      #expect(settings.validationError != nil)
    }
    settings.seed = "18446744073709551615"
    #expect(settings.validationError == nil)
    settings.sampleEvery = 300
    #expect(settings.validationError != nil)
    settings.sampleEvery = 0
    #expect(settings.validationError != nil)
  }

  @Test func exportedCommandsKeepPathsAsLiteralArguments() throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let matrix = directory.appendingPathComponent("a ' $(touch UNEXPECTED) ; matrix.nex")
    let output = directory.appendingPathComponent("output folder")
    let recorder = directory.appendingPathComponent("record arguments")
    try "#!/bin/sh\nprintf '%s\\0' \"$@\"\n".write(to: recorder, atomically: true, encoding: .utf8)
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: recorder.path)
    let command = AnalysisSettings().command(executable: recorder, matrix: matrix, output: output)
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/bin/sh")
    process.arguments = ["-c", command]
    process.currentDirectoryURL = directory
    let pipe = Pipe()
    process.standardOutput = pipe
    try process.run()
    let response = pipe.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    #expect(process.terminationStatus == 0)
    let arguments = String(data: response, encoding: .utf8)?.split(separator: "\0").map(String.init)
    #expect(arguments == AnalysisSettings().arguments(matrix: matrix, output: output))
    #expect(
      !FileManager.default.fileExists(atPath: directory.appendingPathComponent("UNEXPECTED").path))
  }

  @Test func realReaderDecodesReferenceAndMissingData() async throws {
    let reference = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(
      reference, gapsAsMissing: false, executable: executable)
    #expect(report.taxonCount == 20)
    #expect(report.characterCount == 50)
    #expect(report.patternCount == 23)
    #expect(report.samplerError == nil)
    #expect(report.variableError == nil)
    #expect(report.cells.count == 20)
    #expect(report.cells[0].count == 50)
    #expect(report.inputSha256.count == 64)

    let missing = root.appendingPathComponent("fixtures/validation/missing/matrix.nex")
    let degraded = try await MatrixInspector.inspect(
      missing, gapsAsMissing: false, executable: executable)
    #expect(degraded.missingCount == 180)
    #expect(degraded.samplerError == nil)
    #expect(degraded.variableError != nil)

    let draft = AnalysisDraft(matrix: reference, report: report, settings: AnalysisSettings())
    let decoded = try JSONDecoder().decode(AnalysisDraft.self, from: JSONEncoder().encode(draft))
    #expect(decoded.matrixSHA256 == report.inputSha256)
    #expect(decoded.monophyly.isEmpty)
    #expect(decoded.settings == AnalysisSettings())
  }

  @Test @MainActor func importFailurePreservesThePreviousMatrixAndResetRejectsStaleImports()
    async throws
  {
    let workspace = Workspace(executable: executable)
    workspace.load(root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"))
    try await finishImport(workspace)
    let original = try #require(workspace.report)
    #expect(workspace.canOpen(.review))
    workspace.load(root.appendingPathComponent("Cargo.toml"))
    try await finishImport(workspace)
    #expect(workspace.error != nil)
    #expect(workspace.report?.inputSha256 == original.inputSha256)
    workspace.load(URL(string: "https://example.invalid/matrix.nex")!)
    try await finishImport(workspace)
    #expect(workspace.error == "Choose a local NEXUS file from your Mac.")
    #expect(workspace.report?.inputSha256 == original.inputSha256)
    workspace.load(root.appendingPathComponent("fixtures/validation/missing/matrix.nex"))
    try await finishImport(workspace)
    #expect(workspace.canOpen(.model))
    #expect(!workspace.canOpen(.review))
    workspace.settings.conditioning = "none"
    #expect(workspace.canOpen(.review))
    workspace.load(root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"))
    workspace.reset()
    try await Task.sleep(for: .milliseconds(200))
    #expect(workspace.report == nil)
    #expect(workspace.error == nil)
    #expect(!workspace.loading)
  }

  @MainActor private func finishImport(_ workspace: Workspace) async throws {
    for _ in 0..<100 {
      if !workspace.loading { return }
      try await Task.sleep(for: .milliseconds(50))
    }
    throw ImportError.message("Matrix import did not finish within five seconds.")
  }
}
