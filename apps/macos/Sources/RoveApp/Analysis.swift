import Foundation

struct MatrixReport: Decodable, Sendable {
  let schemaVersion: Int
  let inputSha256: String
  let taxonCount: Int
  let characterCount: Int
  let patternCount: Int?
  let missingCount: Int
  let gapCount: Int
  let uncertainCount: Int
  let polymorphicCount: Int
  let samplerError: String?
  let variableError: String?
  let symbols: [String]
  let taxa: [String]
  let taxonNames: [String]?
  let characterLabels: [String?]
  let cells: [[String]]
  var allTaxa: [String] { taxonNames ?? taxa }

  var missingPercent: Double {
    100 * Double(missingCount) / (Double(taxonCount) * Double(characterCount))
  }

  static func decode(_ data: Data) throws -> Self {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let value = try decoder.decode(Self.self, from: data)
    guard value.schemaVersion == 1, value.taxonCount > 0, value.characterCount > 0,
      value.cells.count == value.taxa.count, !value.cells.isEmpty,
      value.cells.allSatisfy({ $0.count == value.characterLabels.count })
    else {
      throw ImportError.message("The matrix preview has an unsupported format.")
    }
    return value
  }
}

enum ImportError: LocalizedError {
  case message(String)
  case readerFailed(String)
  var errorDescription: String? {
    switch self {
    case .message(let text): text
    case .readerFailed(let text): text
    }
  }
}

enum MatrixInspector {
  static var executable: URL {
    Bundle.main.bundleURL.appendingPathComponent("Contents/Helpers/rove")
  }

  static func inspect(_ url: URL, gapsAsMissing: Bool, executable: URL = executable) async throws
    -> MatrixReport
  {
    try await Task.detached(priority: .userInitiated) {
      let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
        UUID().uuidString)
      try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
      defer { try? FileManager.default.removeItem(at: directory) }
      let output = directory.appendingPathComponent("output.json")
      let errors = directory.appendingPathComponent("error.txt")
      FileManager.default.createFile(atPath: output.path, contents: nil)
      FileManager.default.createFile(atPath: errors.path, contents: nil)
      let stdout = try FileHandle(forWritingTo: output)
      let stderr = try FileHandle(forWritingTo: errors)
      defer {
        try? stdout.close()
        try? stderr.close()
      }
      let process = Process()
      process.executableURL = executable
      process.arguments = ["inspect", url.path, "--gaps", gapsAsMissing ? "missing" : "reject"]
      process.standardOutput = stdout
      process.standardError = stderr
      do { try process.run() } catch {
        throw ImportError.message(
          "Rove couldn't open its matrix reader. Rebuild the app and try again.")
      }
      process.waitUntilExit()
      guard process.terminationStatus == 0 else {
        let message =
          (try? String(contentsOf: errors, encoding: .utf8)) ?? "The matrix could not be read."
        throw ImportError.readerFailed(message.trimmingCharacters(in: .whitespacesAndNewlines))
      }
      return try MatrixReport.decode(Data(contentsOf: output))
    }.value
  }
}

struct AnalysisSettings: Codable, Equatable, Sendable {
  var clock = "reference"
  var conditioning = "variable"
  var steps = 10_000_000
  var sampleEvery = 1_000
  var seed = "42"
  var gapsAsMissing = false

  var validationError: String? {
    guard ["strict", "reference"].contains(clock), ["variable", "none"].contains(conditioning)
    else {
      return "This setup uses an unsupported model."
    }
    guard UInt64(seed) != nil else {
      return "Use a whole-number seed from 0 to 18446744073709551615."
    }
    guard steps > 0, sampleEvery > 0, steps % sampleEvery == 0 else {
      return "The sampling interval must divide the number of steps exactly."
    }
    return nil
  }

  func arguments(matrix: URL, output: URL, monophyly: [[String]] = []) -> [String] {
    [
      "sample", matrix.path, "--output", output.path, "--seed", seed,
      "--steps", String(steps), "--sample-every", String(sampleEvery),
      "--model", clock, "--conditioning", conditioning,
      "--gaps", gapsAsMissing ? "missing" : "reject",
    ] + monophyly.flatMap { ["--monophyly", $0.joined(separator: ",")] }
  }

  func command(executable: URL, matrix: URL, output: URL, monophyly: [[String]] = []) -> String {
    ([executable.path] + arguments(matrix: matrix, output: output, monophyly: monophyly)).map {
      value in
      "'" + value.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }.joined(separator: " ")
  }
}

struct AnalysisDraft: Codable, Sendable {
  let formatVersion: Int
  let createdAt: Date
  let matrixPath: String
  let matrixSHA256: String
  let settings: AnalysisSettings
  let monophyly: [[String]]

  init(matrix: URL, report: MatrixReport, settings: AnalysisSettings, monophyly: [[String]] = []) {
    formatVersion = 1
    createdAt = Date()
    matrixPath = matrix.path
    matrixSHA256 = report.inputSha256
    self.settings = settings
    self.monophyly = monophyly
  }

  static func read(_ url: URL) throws -> Self {
    let values = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard url.isFileURL, values.isRegularFile == true, (values.fileSize ?? 0) <= 1_048_576 else {
      throw ImportError.message("Choose a setup file smaller than 1 MB.")
    }
    let decoder = JSONDecoder()
    decoder.dateDecodingStrategy = .iso8601
    let draft = try decoder.decode(Self.self, from: Data(contentsOf: url))
    guard draft.formatVersion == 1, (draft.matrixPath as NSString).isAbsolutePath,
      draft.matrixSHA256.count == 64,
      draft.matrixSHA256.utf8.allSatisfy({
        (48...57).contains($0) || (97...102).contains($0)
      })
    else { throw ImportError.message("This setup has an unsupported or invalid format.") }
    if let issue = draft.settings.validationError { throw ImportError.message(issue) }
    return draft
  }

  func write(to url: URL) throws {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    encoder.dateEncodingStrategy = .iso8601
    try encoder.encode(self).write(to: url, options: .atomic)
  }
}

enum GroupConstraints {
  static func validate(_ groups: [[String]], taxa: [String]) -> String? {
    let known = Set(taxa)
    var previous: [Set<String>] = []
    for group in groups {
      let names = Set(group)
      guard names.count >= 2, names.count == group.count, names.isSubset(of: known) else {
        return "Each group needs at least two distinct terminals from this matrix."
      }
      for other in previous {
        if names == other { return "This group is already constrained." }
        if !names.isDisjoint(with: other), !names.isSubset(of: other), !other.isSubset(of: names) {
          return "Groups must be nested or separate; partially overlapping groups are incompatible."
        }
      }
      previous.append(names)
    }
    return nil
  }
}
