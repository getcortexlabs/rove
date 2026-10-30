import Foundation

struct RunCheckpoint: Decodable, Sendable {
  struct Chain: Decodable, Sendable {
    let step: UInt64
    let file: String
  }
  let schemaVersion: Int
  let inputSha256: String
  let executableSha256: String
  let arguments: [String]
  let chains: [Chain]
  var step: UInt64 { chains.first?.step ?? 0 }

  static func read(directory: URL, draft: AnalysisDraft) throws -> Self {
    let output = directory.appendingPathComponent("results")
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let value = try decoder.decode(
      Self.self, from: RecordFile.read(output.appendingPathComponent("checkpoint.json")))
    let expected = try draft.settings.arguments(
      matrix: URL(fileURLWithPath: draft.matrixPath).resolvingSymlinksInPath(),
      output: output.resolvingSymlinksInPath(), monophyly: draft.constraints)
    var recorded = value.arguments
    // Foundation normalizes /private/var to /var on macOS; Rust's realpath
    // keeps /private. Compare both paths using the same normalization.
    if recorded.count > 3, recorded[2] == "--output" {
      for index in [1, 3] {
        recorded[index] = URL(fileURLWithPath: recorded[index]).resolvingSymlinksInPath().path
      }
    }
    if let index = expected.firstIndex(of: "--dating-json") {
      guard recorded.indices.contains(index + 1), recorded[index] == "--dating-json",
        try EngineDatingModel.decode(recorded[index + 1])
          == EngineDatingModel.decode(expected[index + 1])
      else { throw RunFailure.invalidCheckpoint }
      recorded[index + 1] = expected[index + 1]
    }
    guard value.schemaVersion == 1, value.inputSha256 == draft.matrixSHA256,
      recorded.starts(with: expected), value.chains.count == draft.settings.chains,
      value.step <= UInt64(draft.settings.steps),
      value.step % UInt64(draft.settings.sampleEvery) == 0,
      value.chains.allSatisfy({ $0.step == value.step && $0.file == "checkpoint-\(value.step).bin" }
      )
    else { throw RunFailure.invalidCheckpoint }
    return value
  }
}
