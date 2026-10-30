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

  @Test @MainActor func orderedPartitionsKeepTheirModelsAndRejectInformativeCoding() async throws {
    let matrix = root.appendingPathComponent("fixtures/validation/ordered/matrix.nex")
    let report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, executable: executable)
    let parts = try #require(report.partitions)
    #expect(parts.map(\.states) == [2, 3, 3, 4])
    #expect(parts.map(\.ordering) == [.unordered, .unordered, .ordered, .ordered])
    #expect(parts.last?.stateSymbols == ["0123", "0123"])
    #expect(report.samplerError == nil)
    #expect(report.variableError == nil)
    #expect(report.informativeError?.contains("ordered") == true)
    #expect(AppStrings(language: .english).partitionSummary(parts[2]).contains("Ordered"))
    #expect(AppStrings(language: .portuguese).partitionSummary(parts[2]).contains("Ordenados"))
    let suite = "rove-ordered-tests-\(UUID().uuidString)"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let workspace = Workspace(
      executable: executable, preferences: AppPreferences(defaults: defaults))
    workspace.load(matrix)
    try await finishImport(workspace)
    #expect(workspace.canOpen(.review))
    workspace.settings.conditioning = "informative"
    #expect(!workspace.canOpen(.review))
    workspace.settings.conditioning = "none"
    #expect(workspace.canOpen(.review))
    workspace.settings.partitioning = "declared"
    workspace.reloadGapPolicy()
    try await finishImport(workspace)
    #expect(workspace.report?.partitions?.map(\.states) == [4, 4])
    #expect(workspace.report?.partitions?.map(\.ordering) == [.unordered, .ordered])
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

  @Test @MainActor func multistateImportAndPartitionPolicyPersistThroughSetup() async throws {
    let matrix = root.appendingPathComponent("fixtures/validation/multistate/matrix.nex")
    let report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, executable: executable)
    #expect(report.samplerError == nil)
    #expect(report.variableError == nil)
    #expect(report.partitions?.map(\.states) == [2, 3, 4])
    #expect(report.partitions?.map(\.characters) == [3, 4, 1])
    #expect(report.allTaxa.first == "Canella-A")
    let declared = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, partitioning: "declared", executable: executable)
    #expect(declared.partitions?.map(\.states) == [4])
    let draft = AnalysisDraft(matrix: matrix, report: report, settings: AnalysisSettings())
    let encoded = try JSONEncoder().encode(draft)
    let decoded = try JSONDecoder().decode(AnalysisDraft.self, from: encoded)
    #expect(decoded.settings.partitioning == "observed")
    var legacy = try #require(JSONSerialization.jsonObject(with: encoded) as? [String: Any])
    var settings = try #require(legacy["settings"] as? [String: Any])
    settings.removeValue(forKey: "partitioning")
    legacy["settings"] = settings
    let old = try JSONDecoder().decode(
      AnalysisDraft.self, from: JSONSerialization.data(withJSONObject: legacy))
    #expect(old.settings.partitioning == "declared")

    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let unknown = directory.appendingPathComponent("missing.nex")
    try
      "#NEXUS BEGIN DATA; DIMENSIONS NTAX=3 NCHAR=2; FORMAT SYMBOLS=\"012\"; MATRIX a ?0 b ?1 c ?2; END;"
      .write(to: unknown, atomically: true, encoding: .utf8)
    let workspace = Workspace(executable: executable)
    workspace.load(unknown)
    try await finishImport(workspace)
    #expect(workspace.report?.samplerError != nil)
    workspace.settings.partitioning = "declared"
    workspace.reloadGapPolicy()
    try await finishImport(workspace)
    #expect(workspace.report?.samplerError == nil)
    #expect(workspace.report?.variableError != nil)
    workspace.settings.conditioning = "none"
    #expect(workspace.canOpen(.review))
  }

  @Test @MainActor func informativeCodingValidatesTheMatrixAndSurvivesSaving() async throws {
    let matrix = root.appendingPathComponent("fixtures/validation/informative/matrix.nex")
    let workspace = Workspace(executable: executable)
    workspace.load(matrix)
    try await finishImport(workspace)
    let report = try #require(workspace.report)
    #expect(report.informativeError == nil)
    #expect(report.partitions?.map(\.states) == [2, 3, 4])
    workspace.settings.conditioning = "informative"
    #expect(workspace.canOpen(.review))
    let draft = AnalysisDraft(matrix: matrix, report: report, settings: workspace.settings)
    let decoded = try JSONDecoder().decode(AnalysisDraft.self, from: JSONEncoder().encode(draft))
    #expect(decoded.settings.conditioning == "informative")
    let arguments = decoded.settings.arguments(
      matrix: matrix, output: matrix.deletingLastPathComponent())
    let option = try #require(arguments.firstIndex(of: "--conditioning"))
    #expect(arguments[option + 1] == "informative")

    workspace.settings.sampleEvery = 0
    #expect(!workspace.canOpen(.review))
    workspace.settings = decoded.settings
    workspace.load(root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"))
    try await finishImport(workspace)
    #expect(workspace.report?.informativeError != nil)
    workspace.settings.conditioning = "informative"
    #expect(!workspace.canOpen(.review))
    workspace.settings.conditioning = "variable"
    #expect(workspace.canOpen(.review))
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
