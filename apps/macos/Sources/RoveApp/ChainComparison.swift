import Foundation

struct ChainComparison: Decodable, Sendable {
  struct Statistic: Decodable, Sendable, Identifiable {
    let name: String
    let rhat: Double?
    let rhatReason: String?
    let ess: [Double?]
    let means: [Double]
    let lagLimitReached: [Bool]
    var id: String { name }
  }
  struct Clade: Decodable, Sendable, Identifiable {
    let taxa: [String]
    let frequencies: [Double]
    let difference: Double
    let constrained: Bool
    let constantInEveryChain: Bool
    var id: String { taxa.joined(separator: "\u{0}") }
  }
  let schemaVersion: Int
  let chainCount: Int
  let burninPercent: Int
  let retainedPerChain: Int
  let lastState: UInt64
  let statistics: [Statistic]
  let clades: [Clade]
  let maxUnconstrainedCladeDifference: Double

  static func read(_ url: URL) throws -> Self {
    let info = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard info.isRegularFile == true, (info.fileSize ?? 0) <= 64 * 1024 * 1024 else {
      throw RunFailure.invalidCompletion
    }
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let report = try decoder.decode(Self.self, from: Data(contentsOf: url))
    guard report.schemaVersion == 1, (2...16).contains(report.chainCount),
      (0..<100).contains(report.burninPercent), report.retainedPerChain > 0,
      !report.statistics.isEmpty,
      Set(report.statistics.map(\.name)).count == report.statistics.count,
      report.statistics.allSatisfy({ value in
        value.ess.count == report.chainCount && value.means.count == report.chainCount
          && value.lagLimitReached.count == report.chainCount
          && value.ess.allSatisfy { $0.map { $0.isFinite && $0 > 0 } ?? true }
          && value.means.allSatisfy(\.isFinite)
          && (value.rhat.map { $0.isFinite && $0 >= 0 } ?? true)
      }),
      report.clades.allSatisfy({ value in
        value.frequencies.count == report.chainCount && !value.taxa.isEmpty
          && value.frequencies.allSatisfy { $0.isFinite && (0...1).contains($0) }
          && value.difference.isFinite && (0...1).contains(value.difference)
      }), report.maxUnconstrainedCladeDifference.isFinite,
      (0...1).contains(report.maxUnconstrainedCladeDifference)
    else { throw RunFailure.invalidCompletion }
    return report
  }
}
