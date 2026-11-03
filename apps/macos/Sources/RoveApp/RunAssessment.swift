import Foundation
import Observation

struct RunAssessment: Decodable, Sendable {
  struct Section: Decodable, Sendable, Identifiable {
    let id: String
    let status: String
    let message: String
  }
  let schemaVersion: Int
  let kind: String
  let createdUnixSeconds: Double
  let source: String
  let chainCount: Int
  let burninPercent: Int
  let originalMatrixStatus: String
  let sections: [Section]
  let treeReports: [String]
  let predictiveReports: [String]
  let convergenceCertified: Bool

  static func read(_ directory: URL) throws -> Self {
    let file = directory.appendingPathComponent("assessment.json")
    let info = try file.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard info.isRegularFile == true, (info.fileSize ?? 0) <= 128 * 1024 * 1024 else {
      throw RunFailure.invalidCompletion
    }
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let value = try decoder.decode(Self.self, from: Data(contentsOf: file))
    guard value.schemaVersion == 1, value.kind == "run_assessment",
      value.createdUnixSeconds.isFinite, (1...16).contains(value.chainCount),
      (0..<100).contains(value.burninPercent), !value.convergenceCertified,
      Set(value.sections.map(\.id))
        == Set(["completion", "sampling", "agreement", "trees", "prediction"]),
      value.sections.count == 5,
      value.sections.allSatisfy({
        ["passed", "attention", "unavailable", "review_required"].contains($0.status)
      })
    else { throw RunFailure.invalidCompletion }
    return value
  }
}

struct AssessmentOptions: Codable, Sendable {
  var burnin = "10"
  var includeMatrix = false
  var valid: Bool { Int(burnin).map { (0..<100).contains($0) } == true }
}

struct AssessmentRequest: Sendable {
  let entry: HistoryEntry
  let executable: URL
  let output: URL
  let options: AssessmentOptions
  let export: Bool

  func execute(with tool: EngineTool) async throws -> RunAssessment {
    guard options.valid else { throw ImportError.message("Choose a burn-in from 0 to 99 percent.") }
    var arguments = [
      export ? "export-run" : "assess", entry.directory.path,
      "--output", output.path, "--burnin-percent", options.burnin, "--cancel-on-stdin-close",
    ]
    if export && options.includeMatrix {
      guard let draft = entry.draft else { throw RunFailure.invalidCompletion }
      arguments += ["--include-matrix", draft.matrixPath]
    }
    let outcome = try await tool.execute(
      executable: executable, arguments: arguments, onLine: { _ in })
    guard outcome.status == 0 else { throw ImportError.readerFailed(outcome.log) }
    let report = try RunAssessment.read(output)
    guard
      URL(fileURLWithPath: report.source).standardizedFileURL.resolvingSymlinksInPath().path
        == entry.directory.standardizedFileURL.resolvingSymlinksInPath().path
    else { throw RunFailure.invalidCompletion }
    return report
  }
}

@MainActor @Observable
final class RunAssessmentModel {
  let entry: HistoryEntry
  let executable: URL
  var options = AssessmentOptions()
  private(set) var busy = false
  private(set) var report: RunAssessment?
  private(set) var reportDirectory: URL?
  private(set) var exported: URL?
  private(set) var error: String?
  private(set) var details: String?
  private var tool: EngineTool?
  private var task: Task<Void, Never>?
  private var parent: URL { entry.directory.appendingPathComponent("assessments") }

  init(entry: HistoryEntry, executable: URL) {
    self.entry = entry
    self.executable = executable
  }

  func load() async {
    guard !busy else { return }
    let directory = parent
    let loaded = await Task.detached {
      let options = try? JSONDecoder().decode(
        AssessmentOptions.self,
        from: RecordFile.read(directory.appendingPathComponent("options.json")))
      let children =
        (try? FileManager.default.contentsOfDirectory(
          at: directory,
          includingPropertiesForKeys: nil)) ?? []
      let reports = children.compactMap { file -> (URL, RunAssessment)? in
        guard let value = try? RunAssessment.read(file) else { return nil }
        return (file, value)
      }.sorted { $0.1.createdUnixSeconds > $1.1.createdUnixSeconds }
      return (options, reports.first)
    }.value
    if let restored = loaded.0, restored.valid { options = restored }
    if let saved = loaded.1,
      URL(fileURLWithPath: saved.1.source).standardizedFileURL.resolvingSymlinksInPath().path
        == entry.directory.standardizedFileURL.resolvingSymlinksInPath().path
    {
      reportDirectory = saved.0
      report = saved.1
    }
  }

  func start(exportTo destination: URL? = nil) {
    guard !busy, options.valid else { return }
    busy = true
    error = nil
    details = nil
    exported = nil
    let tool = EngineTool()
    self.tool = tool
    let request = AssessmentRequest(
      entry: entry, executable: executable,
      output: destination ?? parent.appendingPathComponent("review-\(UUID())"),
      options: options, export: destination != nil)
    task = Task {
      do {
        try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
        try JSONEncoder().encode(request.options).write(
          to: parent.appendingPathComponent("options.json"), options: .atomic)
        let value = try await request.execute(with: tool)
        if destination != nil {
          exported = request.output
        } else {
          report = value
          reportDirectory = request.output
        }
      } catch is CancellationError {
        error = "The review or export was cancelled."
      } catch {
        self.error = "The review or export could not be completed."
        details = error.localizedDescription
      }
      busy = false
      self.tool = nil
    }
  }

  func cancel() { tool?.cancel() }
  func waitForExit() async { await task?.value }
}
