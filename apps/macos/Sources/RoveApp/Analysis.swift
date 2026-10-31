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
  enum Ordering: String, Decodable, Sendable { case ordered, unordered }
  let ordering: Ordering?
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

/// A selection weight or starting step size for one engine move.
struct MoveOverride: Codable, Equatable, Sendable, Identifiable {
  var move: String
  var value: Double
  var id: String { move }
}

/// The engine's move names and the schedules evaluated in the sampling reports.
enum ProposalSchedule {
  struct Preset: Identifiable, Sendable {
    let id: String
    let name: String
    let report: String
    let weights: [MoveOverride]
  }

  static let moves = [
    "node_height", "subtree_slide", "narrow_exchange", "wide_exchange", "wilson_balding",
    "root_scale", "tree_scale", "birth_difference_scale", "relative_death_scale",
    "gamma_shape_scale", "clock_sigma_scale", "branch_rate_scale", "node_time_rates",
    "root_time_rates", "clock_sigma_rates", "tip_age", "clock_rate_scale", "fbd_birth_scale",
    "fbd_death_scale", "fbd_sampling_scale", "fbd_rho_logit", "fbd_origin_scale",
  ]
  static let relaxedClockMoves: Set<String> = [
    "gamma_shape_scale", "clock_sigma_scale", "branch_rate_scale", "node_time_rates",
    "root_time_rates", "clock_sigma_rates",
  ]
  static let datingMoves: Set<String> = [
    "tip_age", "clock_rate_scale", "fbd_birth_scale", "fbd_death_scale", "fbd_sampling_scale",
    "fbd_rho_logit", "fbd_origin_scale",
  ]
  static let undatedMoves: Set<String> = ["birth_difference_scale", "relative_death_scale"]
  /// Moves with a tunable step: a slide size for `subtree_slide`, otherwise a
  /// scale factor strictly between zero and one.
  static let tunable: Set<String> = [
    "subtree_slide", "root_scale", "tree_scale", "birth_difference_scale",
    "relative_death_scale", "gamma_shape_scale", "clock_sigma_scale", "branch_rate_scale",
    "root_time_rates", "clock_sigma_rates", "clock_rate_scale", "fbd_birth_scale",
    "fbd_death_scale", "fbd_sampling_scale", "fbd_rho_logit", "fbd_origin_scale",
  ]
  static let presets = [
    Preset(
      id: "gamma-3", name: "Gamma shape weight 3", report: "#42",
      weights: [MoveOverride(move: "gamma_shape_scale", value: 3)]),
    Preset(
      id: "joint", name: "Joint relaxed-clock moves", report: "#27",
      weights: [
        MoveOverride(move: "node_height", value: 18),
        MoveOverride(move: "node_time_rates", value: 12),
        MoveOverride(move: "root_scale", value: 1),
        MoveOverride(move: "root_time_rates", value: 2),
        MoveOverride(move: "clock_sigma_scale", value: 1),
        MoveOverride(move: "clock_sigma_rates", value: 2),
      ]),
  ]

  static func validate(
    weights: [MoveOverride], tuningValues: [MoveOverride], clock: String, dating: DatingSettings
  ) -> String? {
    for list in [weights, tuningValues] {
      guard Set(list.map(\.move)).count == list.count else {
        return "Each move appears once in the proposal schedule."
      }
      guard list.allSatisfy({ moves.contains($0.move) }) else {
        return "The proposal schedule names a move the engine does not have."
      }
    }
    guard weights.allSatisfy({ $0.value.isFinite && $0.value >= 0 }) else {
      return "Selection weights must be finite and not negative."
    }
    guard
      tuningValues.allSatisfy({
        tunable.contains($0.move) && $0.value.isFinite && $0.value > 0
          && ($0.move == "subtree_slide" || $0.value < 1)
      })
    else { return "Step sizes must be positive; scale factors must be below 1." }
    let active = Set(weights.filter { $0.value > 0 }.map(\.move)).union(tuningValues.map(\.move))
    if clock != "reference", !active.isDisjoint(with: relaxedClockMoves) {
      return "Gamma shape, clock sigma, branch rate and joint moves need the relaxed clock."
    }
    if !dating.enabled, !active.isDisjoint(with: datingMoves) {
      return "Tip age, clock rate and FBD moves need fossil dating."
    }
    if dating.enabled, !active.isDisjoint(with: undatedMoves) {
      return "Birth-difference and relative-death moves need the undated tree prior."
    }
    if dating.enabled, dating.clock == .fixed, active.contains("clock_rate_scale") {
      return "The clock rate move needs an estimated clock rate."
    }
    if active.contains(where: { changesFixedParameter($0, dating.estimation) }) {
      return "FBD process moves need their parameter estimated."
    }
    return nil
  }

  /// Whether `move` scales an FBD parameter that `estimation` keeps fixed.
  static func changesFixedParameter(_ move: String, _ estimation: ProcessEstimation) -> Bool {
    switch move {
    case "fbd_birth_scale": !estimation.birth.estimated
    case "fbd_death_scale": !estimation.death.estimated
    case "fbd_sampling_scale": !estimation.fossilSampling.estimated
    case "fbd_rho_logit": !estimation.estimateSampling
    case "fbd_origin_scale": !estimation.origin.estimated
    default: false
    }
  }
}

struct AnalysisSettings: Codable, Equatable, Sendable {
  var dating = DatingSettings()
  var clock = "reference"
  var conditioning = "variable"
  var partitioning = "observed"
  var steps = 10_000_000
  var sampleEvery = 1_000
  var seed = "42"
  var gapsAsMissing = false
  var chains = 1
  var stopWhenConverged = false
  /// Ignore the characters and sample the priors alone.
  var sampleFromPrior = false
  /// Stopping thresholds; nil keeps the engine's defaults.
  var minESS: Double? = nil
  var maxRhat: Double? = nil
  var checkEvery: Int? = nil
  /// Checkpoint interval in steps; nil keeps the engine's time-based default.
  var checkpointEvery: Int? = nil
  var tuning = true
  var weights: [MoveOverride] = []
  var tuningValues: [MoveOverride] = []

  init() {}

  private enum CodingKeys: String, CodingKey {
    case dating, clock, conditioning, partitioning, steps, sampleEvery, seed, gapsAsMissing, chains,
      stopWhenConverged, sampleFromPrior, minESS, maxRhat, checkEvery, checkpointEvery, tuning,
      weights, tuningValues
  }

  init(from decoder: Decoder) throws {
    let values = try decoder.container(keyedBy: CodingKeys.self)
    dating = try values.decodeIfPresent(DatingSettings.self, forKey: .dating) ?? DatingSettings()
    clock = try values.decode(String.self, forKey: .clock)
    conditioning = try values.decode(String.self, forKey: .conditioning)
    partitioning = try values.decodeIfPresent(String.self, forKey: .partitioning) ?? "declared"
    steps = try values.decode(Int.self, forKey: .steps)
    sampleEvery = try values.decode(Int.self, forKey: .sampleEvery)
    seed = try values.decode(String.self, forKey: .seed)
    gapsAsMissing = try values.decode(Bool.self, forKey: .gapsAsMissing)
    chains = try values.decodeIfPresent(Int.self, forKey: .chains) ?? 1
    stopWhenConverged = try values.decodeIfPresent(Bool.self, forKey: .stopWhenConverged) ?? false
    sampleFromPrior = try values.decodeIfPresent(Bool.self, forKey: .sampleFromPrior) ?? false
    minESS = try values.decodeIfPresent(Double.self, forKey: .minESS)
    maxRhat = try values.decodeIfPresent(Double.self, forKey: .maxRhat)
    checkEvery = try values.decodeIfPresent(Int.self, forKey: .checkEvery)
    checkpointEvery = try values.decodeIfPresent(Int.self, forKey: .checkpointEvery)
    tuning = try values.decodeIfPresent(Bool.self, forKey: .tuning) ?? true
    weights = try values.decodeIfPresent([MoveOverride].self, forKey: .weights) ?? []
    tuningValues = try values.decodeIfPresent([MoveOverride].self, forKey: .tuningValues) ?? []
  }

  /// Whether any option beyond the exploration plan is in effect; such setups
  /// use format 5 so that earlier app versions do not run them without it.
  /// Thresholds are kept but inert while automatic stopping is off.
  var usesAdvancedOptions: Bool {
    sampleFromPrior || (stopWhenConverged && (minESS != nil || maxRhat != nil || checkEvery != nil))
      || checkpointEvery != nil || !tuning || !weights.isEmpty || !tuningValues.isEmpty
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
    if let value = minESS, !(value.isFinite && value > 0) {
      return "The minimum ESS must be a positive number."
    }
    if let value = maxRhat, !(value.isFinite && value >= 1) {
      return "The maximum R-hat must be at least 1."
    }
    for value in [checkEvery, checkpointEvery].compactMap({ $0 })
    where value <= 0 || value % sampleEvery != 0 {
      return "Check and checkpoint intervals must be positive multiples of the sampling interval."
    }
    if let issue = ProposalSchedule.validate(
      weights: weights, tuningValues: tuningValues, clock: clock, dating: dating)
    {
      return issue
    }
    return dating.validationError()
  }

  func arguments(matrix: URL, output: URL, monophyly: [[String]] = [], datingFile: URL? = nil)
    throws -> [String]
  {
    var arguments: [String] = [
      "sample", matrix.path, "--output", output.path, "--seed", seed,
      "--steps", String(steps), "--sample-every", String(sampleEvery),
    ]
    if chains > 1 { arguments += ["--chains", String(chains)] }
    if stopWhenConverged {
      arguments.append("--stop-when-converged")
      if let minESS { arguments += ["--min-ess", String(minESS)] }
      if let maxRhat { arguments += ["--max-rhat", String(maxRhat)] }
      if let checkEvery { arguments += ["--check-every", String(checkEvery)] }
    }
    if let checkpointEvery { arguments += ["--checkpoint-every", String(checkpointEvery)] }
    if sampleFromPrior { arguments.append("--sample-from-prior") }
    return arguments + (try modelArguments(monophyly: monophyly, datingFile: datingFile))
  }

  /// The model options shared by `rove sample` and `rove marginal`.
  func modelArguments(monophyly: [[String]] = [], datingFile: URL? = nil) throws -> [String] {
    var arguments = [
      "--model", clock, "--conditioning", conditioning,
      "--gaps", gapsAsMissing ? "missing" : "reject", "--partitioning", partitioning,
    ]
    if !tuning { arguments.append("--no-tuning") }
    for override in weights {
      arguments += ["--weight", "\(override.move)=\(String(override.value))"]
    }
    for override in tuningValues {
      arguments += ["--tuning", "\(override.move)=\(String(override.value))"]
    }
    for clade in monophyly { arguments += ["--monophyly", clade.joined(separator: ",")] }
    if let model = try dating.model() {
      arguments += try datingFile.map { ["--dating", $0.path] } ?? ["--dating-json", model.json()]
    }
    return arguments
  }

  func command(executable: URL, matrix: URL, output: URL, monophyly: [[String]] = []) throws
    -> String
  {
    ([executable.path] + (try arguments(matrix: matrix, output: output, monophyly: monophyly))).map
    {
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
    formatVersion =
      settings.usesAdvancedOptions
      ? 5 : (settings.dating.estimatesProcess ? 4 : (settings.dating.enabled ? 3 : 2))
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
    guard [1, 2, 3, 4, 5].contains(formatVersion),
      formatVersion >= 5 || !settings.usesAdvancedOptions,
      formatVersion >= 4 || !settings.dating.estimatesProcess,
      formatVersion >= 3 || !settings.dating.enabled, (matrixPath as NSString).isAbsolutePath,
      matrixSHA256.count == 64,
      matrixSHA256.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) })
    else { throw ImportError.message("This setup has an unsupported or invalid format.") }
    if let issue = settings.validationError { throw ImportError.message(issue) }
    if let issue = settings.dating.validationError(taxa: taxa) { throw ImportError.message(issue) }
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
    if let issue = settings.dating.validationError(taxa: actual) {
      throw ImportError.message(issue)
    }
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
