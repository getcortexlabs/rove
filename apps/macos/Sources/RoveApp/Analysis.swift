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
  let informativeError: String?
  let symbols: [String]
  let taxa: [String]
  let taxonNames: [String]?
  let characterLabels: [String?]
  let cells: [[String]]
  let partitions: [CharacterPartition]?
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

struct CharacterPartition: Decodable, Sendable {
  let states: Int
  let characters: Int
  let observedPatterns: Int
  let sites: [Int]
  let stateSymbols: [String]
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

  static func inspect(
    _ url: URL, gapsAsMissing: Bool, partitioning: String = "observed", executable: URL = executable
  ) async throws
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
      process.arguments = [
        "inspect", url.path, "--gaps", gapsAsMissing ? "missing" : "reject", "--partitioning",
        partitioning,
      ]
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
  var partitioning = "observed"
  var steps = 10_000_000
  var sampleEvery = 1_000
  var seed = "42"
  var gapsAsMissing = false
  var chains = 1
  var stopWhenConverged = false

  init() {}

  private enum CodingKeys: String, CodingKey {
    case clock, conditioning, partitioning, steps, sampleEvery, seed, gapsAsMissing, chains,
      stopWhenConverged
  }

  init(from decoder: Decoder) throws {
    let values = try decoder.container(keyedBy: CodingKeys.self)
    clock = try values.decode(String.self, forKey: .clock)
    conditioning = try values.decode(String.self, forKey: .conditioning)
    partitioning = try values.decodeIfPresent(String.self, forKey: .partitioning) ?? "declared"
    steps = try values.decode(Int.self, forKey: .steps)
    sampleEvery = try values.decode(Int.self, forKey: .sampleEvery)
    seed = try values.decode(String.self, forKey: .seed)
    gapsAsMissing = try values.decode(Bool.self, forKey: .gapsAsMissing)
    chains = try values.decodeIfPresent(Int.self, forKey: .chains) ?? 1
    stopWhenConverged = try values.decodeIfPresent(Bool.self, forKey: .stopWhenConverged) ?? false
  }

  var validationError: String? {
    guard ["strict", "reference"].contains(clock),
      ["variable", "informative", "none"].contains(conditioning),
      ["observed", "declared"].contains(partitioning)
    else {
      return "This setup uses an unsupported model."
    }
    guard [1, 2, 4].contains(chains), !stopWhenConverged || chains > 1 else {
      return "Choose one, two or four chains. Automatic stopping needs multiple chains."
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
      "--partitioning", partitioning,
    ] + (chains > 1 ? ["--chains", String(chains)] : [])
      + (stopWhenConverged ? ["--stop-when-converged"] : [])
      + monophyly.flatMap { ["--monophyly", $0.joined(separator: ",")] }
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
  let outgroup: String?
  let taxa: [String]?
  let appliedMonophyly: [[String]]?
  var constraints: [[String]] {
    GroupConstraints.applied(monophyly, taxa: taxa ?? [], outgroup: outgroup)
  }

  init(
    matrix: URL, report: MatrixReport, settings: AnalysisSettings,
    monophyly: [[String]] = [], outgroup: String? = nil
  ) {
    formatVersion = 2
    createdAt = Date()
    matrixPath = matrix.path
    matrixSHA256 = report.inputSha256
    self.settings = settings
    self.monophyly = monophyly
    self.outgroup = outgroup
    taxa = report.allTaxa
    appliedMonophyly = GroupConstraints.applied(monophyly, taxa: report.allTaxa, outgroup: outgroup)
  }

  static func read(_ url: URL) throws -> Self {
    let values = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard url.isFileURL, values.isRegularFile == true, (values.fileSize ?? 0) <= 1_048_576 else {
      throw ImportError.message("Choose a setup file smaller than 1 MB.")
    }
    let decoder = JSONDecoder()
    decoder.dateDecodingStrategy = .iso8601
    let draft = try decoder.decode(Self.self, from: Data(contentsOf: url))
    try draft.validate()
    return draft
  }

  func validate() throws {
    guard [1, 2].contains(formatVersion), (matrixPath as NSString).isAbsolutePath,
      matrixSHA256.count == 64,
      matrixSHA256.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) })
    else { throw ImportError.message("This setup has an unsupported or invalid format.") }
    if let issue = settings.validationError { throw ImportError.message(issue) }
    if formatVersion == 1 {
      guard outgroup == nil else {
        throw ImportError.message("This setup has an unsupported or invalid format.")
      }
    } else {
      guard let taxa, !taxa.isEmpty, Set(taxa).count == taxa.count, appliedMonophyly == constraints
      else {
        throw ImportError.message("This setup has an unsupported or invalid format.")
      }
      if let issue = GroupConstraints.validate(monophyly, taxa: taxa, outgroup: outgroup) {
        throw ImportError.message(issue)
      }
    }
  }

  func validateTaxa(_ actual: [String]) throws {
    if let taxa, taxa != actual {
      throw ImportError.message("The saved terminals do not match the original matrix.")
    }
    if let issue = GroupConstraints.validate(monophyly, taxa: actual, outgroup: outgroup) {
      throw ImportError.message(issue)
    }
  }

  func write(to url: URL) throws {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    encoder.dateEncodingStrategy = .iso8601
    try encoder.encode(self).write(to: url, options: .atomic)
  }
}

enum GroupConstraints {
  // A single outgroup is sister to the clade containing every other terminal.
  static func applied(_ groups: [[String]], taxa: [String], outgroup: String?) -> [[String]] {
    guard let outgroup else { return groups }
    let ingroup = taxa.filter { $0 != outgroup }
    return groups.contains(where: { Set($0) == Set(ingroup) }) ? groups : [ingroup] + groups
  }

  static func validate(_ groups: [[String]], taxa: [String], outgroup: String? = nil) -> String? {
    let known = Set(taxa)
    if let outgroup, !known.contains(outgroup) || known.count < 3 {
      return "Choose one outgroup from a matrix with at least three terminals."
    }
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
    if outgroup != nil {
      return validate(applied(groups, taxa: taxa, outgroup: outgroup), taxa: taxa)
    }
    return nil
  }
}

struct GroupSelection: Identifiable {
  let id = UUID()
  let index: Int?
  var selected: Set<String>

  init(index: Int? = nil, group: [String] = []) {
    self.index = index
    selected = Set(group)
  }

  func groups(replacing groups: [[String]], taxa: [String]) -> [[String]] {
    var result = groups
    let group = taxa.filter { selected.contains($0) }
    if let index, result.indices.contains(index) {
      result[index] = group
    } else {
      result.append(group)
    }
    return result
  }
}
