import Foundation

// Optional timing fields keep session files written by the first execution UI readable.
struct SessionRecord: Codable, Sendable {
  let formatVersion: Int
  let id: UUID
  let status: String
  let setup: AnalysisDraft
  let exitCode: Int32?
  let error: String?
  let startedAt: Date?
  let finishedAt: Date?
  let elapsedSeconds: Double?
  let completedSteps: UInt64?

  static func read(_ url: URL) throws -> Self {
    let decoder = JSONDecoder()
    decoder.dateDecodingStrategy = .iso8601
    let value = try decoder.decode(Self.self, from: RecordFile.read(url))
    guard value.formatVersion == 1,
      ["partial", "completed", "paused", "cancelled", "failed"].contains(value.status),
      value.elapsedSeconds.map({ $0.isFinite && $0 >= 0 }) ?? true,
      value.completedSteps.map({ $0 <= UInt64(max(0, value.setup.settings.steps)) }) ?? true
    else { throw RunFailure.invalidCompletion }
    try value.setup.validate()
    return value
  }
}

struct EngineRecord: Decodable, Sendable {
  struct Input: Decodable, Sendable {
    let sha256: String
    let gaps: String
  }
  struct Settings: Decodable, Sendable {
    let steps: UInt64
    let sampleEvery: UInt64
    let seed: UInt64
    var tuning: Bool? = nil
  }
  struct Model: Decodable, Sendable {
    let dating: EngineDatingModel?
    let monophyly: [[String]]
    let preset: String
    let partitioning: String?
    let partitions: [CharacterPartition]?
    var sampleFromPrior: Bool? = nil
  }
  struct MoveRecord: Decodable, Sendable {
    let name: String
    let weight: Double
    let initialTuning: Double?
  }
  let status: String
  let completedSteps: UInt64
  let input: Input
  let settings: Settings
  let model: Model
  let command: [String]
  let startedUnixSeconds: Double?
  let wallSeconds: Double?
  let kind: String?
  let chainCount: Int?
  let terminationReason: String?
  let chains: [Chain]?
  var moves: [MoveRecord]? = nil

  struct Chain: Decodable, Sendable {
    let index: Int
    let seed: UInt64
    let directory: String
    let status: String?
    let completedSteps: UInt64?
  }

  static func read(_ url: URL) throws -> Self {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    return try decoder.decode(Self.self, from: RecordFile.read(url))
  }

  func matches(_ draft: AnalysisDraft, seed: UInt64? = nil) -> Bool {
    guard let conditioningIndex = command.firstIndex(of: "--conditioning"),
      command.indices.contains(conditioningIndex + 1)
    else { return false }
    let expectedDating: EngineDatingModel?
    do { expectedDating = try draft.settings.dating.model() } catch { return false }
    return draft.settings.validationError == nil && input.sha256 == draft.matrixSHA256
      && model.dating == expectedDating
      && input.gaps == (draft.settings.gapsAsMissing ? "missing" : "reject")
      && model.preset == draft.settings.clock
      && (model.partitioning ?? "declared") == draft.settings.partitioning
      && command[conditioningIndex + 1] == draft.settings.conditioning
      && settings.steps == UInt64(draft.settings.steps)
      && settings.sampleEvery == UInt64(draft.settings.sampleEvery)
      && settings.seed == (seed ?? UInt64(draft.settings.seed))
      && model.monophyly == draft.constraints
      && completedSteps <= settings.steps
      && (model.sampleFromPrior ?? false) == draft.settings.sampleFromPrior
      && (settings.tuning ?? true) == draft.settings.tuning
      && (kind == "independent_chains" || schedule(matches: draft.settings))
  }

  /// Group records summarize chain-01 without the move table; each chain's
  /// own record carries it.
  private func schedule(matches settings: AnalysisSettings) -> Bool {
    settings.weights.allSatisfy { override in
      moves?.first { $0.name == override.move }?.weight == override.value
    }
      && settings.tuningValues.allSatisfy { override in
        moves?.first { $0.name == override.move }?.initialTuning == override.value
      }
  }

  static func verify(directory: URL, draft: AnalysisDraft) throws -> Self {
    let output = directory.appendingPathComponent("results")
    let record = try read(output.appendingPathComponent("run.json"))
    guard record.matches(draft), ["completed", "paused", "cancelled"].contains(record.status) else {
      throw RunFailure.invalidCompletion
    }
    func check(_ value: Self, at folder: URL) throws {
      guard value.completedSteps <= value.settings.steps,
        value.status != "completed" || value.completedSteps == value.settings.steps
          || (draft.settings.stopWhenConverged && value.terminationReason == "diagnostic_thresholds"
            && value.completedSteps > 0 && value.completedSteps % value.settings.sampleEvery == 0)
      else { throw RunFailure.invalidCompletion }
      for file in ["trace.log", "trees.nexus"] {
        let info = try folder.appendingPathComponent(file).resourceValues(forKeys: [
          .isRegularFileKey, .fileSizeKey,
        ])
        guard info.isRegularFile == true, (info.fileSize ?? 0) > 0 else {
          throw RunFailure.invalidCompletion
        }
      }
    }
    if draft.settings.chains == 1 {
      guard record.kind == nil, record.chains == nil else { throw RunFailure.invalidCompletion }
      try check(record, at: output)
    } else {
      guard record.kind == "independent_chains", record.chainCount == draft.settings.chains,
        let members = record.chains, members.count == draft.settings.chains,
        let seed = UInt64(draft.settings.seed)
      else { throw RunFailure.invalidCompletion }
      var minimum = record.settings.steps
      for (offset, member) in members.enumerated() {
        guard member.index == offset + 1, member.seed == seed &+ UInt64(offset),
          member.directory == String(format: "chain-%02d", offset + 1)
        else { throw RunFailure.invalidCompletion }
        let folder = output.appendingPathComponent(member.directory)
        let child = try read(folder.appendingPathComponent("run.json"))
        guard child.matches(draft, seed: member.seed), child.status == member.status,
          child.completedSteps == member.completedSteps,
          ["completed", "paused", "cancelled"].contains(child.status),
          record.status != "completed"
            || (child.status == "completed"
              && child.completedSteps == record.completedSteps
              && child.terminationReason == record.terminationReason)
        else { throw RunFailure.invalidCompletion }
        try check(child, at: folder)
        minimum = min(minimum, child.completedSteps)
      }
      guard minimum == record.completedSteps else { throw RunFailure.invalidCompletion }
      if record.status == "completed" {
        let comparison = try ChainComparison.read(output.appendingPathComponent("comparison.json"))
        guard comparison.chainCount == members.count, comparison.lastState == record.completedSteps
        else {
          throw RunFailure.invalidCompletion
        }
      }
    }
    return record
  }

}

enum RecordFile {
  static func read(_ url: URL) throws -> Data {
    let values = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard values.isRegularFile == true, (values.fileSize ?? 0) <= 2 * 1024 * 1024 else {
      throw RunFailure.invalidCompletion
    }
    return try Data(contentsOf: url)
  }
}

// Chain analysis tools keep their derived files beside that chain. The session
// remains the owner of completion status and the original analysis setup.
enum RunFiles {
  static func isChain(_ directory: URL) -> Bool {
    directory.deletingLastPathComponent().lastPathComponent == "results"
      && directory.lastPathComponent.hasPrefix("chain-")
  }
  static func output(in directory: URL) -> URL {
    isChain(directory) ? directory : directory.appendingPathComponent("results")
  }
  static func session(in directory: URL) -> URL {
    isChain(directory)
      ? directory.deletingLastPathComponent().deletingLastPathComponent() : directory
  }
}
