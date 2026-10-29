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
      ["partial", "completed", "cancelled", "failed"].contains(value.status),
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
  }
  struct Model: Decodable, Sendable {
    let monophyly: [[String]]
    let preset: String
  }
  let status: String
  let completedSteps: UInt64
  let input: Input
  let settings: Settings
  let model: Model
  let command: [String]
  let startedUnixSeconds: Double?
  let wallSeconds: Double?

  static func read(_ url: URL) throws -> Self {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    return try decoder.decode(Self.self, from: RecordFile.read(url))
  }

  func matches(_ draft: AnalysisDraft) -> Bool {
    guard let conditioningIndex = command.firstIndex(of: "--conditioning"),
      command.indices.contains(conditioningIndex + 1)
    else { return false }
    return draft.settings.validationError == nil && input.sha256 == draft.matrixSHA256
      && input.gaps == (draft.settings.gapsAsMissing ? "missing" : "reject")
      && model.preset == draft.settings.clock
      && command[conditioningIndex + 1] == draft.settings.conditioning
      && settings.steps == UInt64(draft.settings.steps)
      && settings.sampleEvery == UInt64(draft.settings.sampleEvery)
      && settings.seed == UInt64(draft.settings.seed) && model.monophyly == draft.monophyly
      && completedSteps <= settings.steps
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
