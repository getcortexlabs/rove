import Charts
import SwiftUI

struct PredictiveDistribution: Sendable {
  struct Bin: Identifiable, Sendable {
    let id: Int
    let lower: Double
    let upper: Double
    let count: Int
  }
  let name: String
  let differences: [Double]
  let undefined: Int

  var bins: [Bin] {
    guard let low = differences.min(), let high = differences.max() else { return [] }
    let width = high == low ? max(abs(low) * 0.05, 0.1) : (high - low) / 20
    guard width.isFinite && width > 0 else { return [] }
    let start = high == low ? low - width / 2 : low
    let count = high == low ? 1 : 20
    var totals = [Int](repeating: 0, count: count)
    for value in differences { totals[min(count - 1, max(0, Int((value - start) / width)))] += 1 }
    return totals.indices.map {
      Bin(
        id: $0, lower: start + Double($0) * width,
        upper: start + Double($0 + 1) * width, count: totals[$0])
    }
  }

  static func read(_ prediction: SavedPrediction) throws -> [Self] {
    struct Pair: Decodable {
      let replicate: Int
      let observed: [String: Double?]
      let simulated: [String: Double?]
    }
    let file = prediction.directory.appendingPathComponent("replicates.jsonl")
    let info = try file.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard info.isRegularFile == true, (info.fileSize ?? 0) <= 64 * 1024 * 1024 else {
      throw RunFailure.invalidCompletion
    }
    let data = try Data(contentsOf: file)
    let pairs = try data.split(separator: 10).map {
      try JSONDecoder().decode(Pair.self, from: Data($0))
    }
    guard pairs.count == prediction.report.replicates,
      Set(pairs.map(\.replicate)) == Set(1...prediction.report.replicates)
    else { throw RunFailure.invalidCompletion }
    return try PredictiveReport.statistics.map { name in
      let differences = try pairs.compactMap { pair -> Double? in
        guard pair.observed.keys.contains(name), pair.simulated.keys.contains(name) else {
          throw RunFailure.invalidCompletion
        }
        guard let a = pair.observed[name] ?? nil, let b = pair.simulated[name] ?? nil else {
          return nil
        }
        let difference = b - a
        guard a.isFinite, b.isFinite, difference.isFinite else {
          throw RunFailure.invalidCompletion
        }
        return difference
      }
      guard let tail = prediction.report.global[name], tail.validPairs == differences.count,
        tail.undefinedPairs == pairs.count - differences.count,
        tail.lowerCount == differences.filter({ $0 <= 0 }).count,
        tail.upperCount == differences.filter({ $0 >= 0 }).count,
        tail.ties == differences.filter({ $0 == 0 }).count
      else { throw RunFailure.invalidCompletion }
      return Self(name: name, differences: differences, undefined: pairs.count - differences.count)
    }
  }
}

struct PredictiveDistributionView: View {
  let prediction: SavedPrediction
  let strings: AppStrings
  @State private var distributions: [PredictiveDistribution] = []
  @State private var selected = "parsimony_length"
  @State private var failed = false

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Picker(strings("Statistic"), selection: $selected) {
        ForEach(PredictiveReport.statistics, id: \.self) { name in
          Text(strings(PredictiveReportView.label(name))).tag(name)
        }
      }.frame(maxWidth: 360)
      if let distribution = distributions.first(where: { $0.name == selected }) {
        if !distribution.differences.isEmpty {
          Chart {
            ForEach(distribution.bins) { bin in
              RectangleMark(
                xStart: .value("Difference", bin.lower), xEnd: .value("Difference", bin.upper),
                yStart: .value("Pairs", 0), yEnd: .value("Pairs", bin.count)
              )
              .foregroundStyle(Lab.lilac.opacity(0.7))
            }
            RuleMark(x: .value("Zero", 0)).lineStyle(StrokeStyle(lineWidth: 1, dash: [4, 3]))
              .foregroundStyle(Lab.secondary)
          }.frame(height: 180)
            .chartXAxisLabel(strings("Simulated − observed"))
            .chartYAxisLabel(strings("Valid pairs"))
            .accessibilityLabel(
              strings(
                "Distribution of simulated minus observed paired statistics. Zero is equality."))
        }
        Text(
          strings(
            "%@ valid pairs · %@ undefined", String(distribution.differences.count),
            String(distribution.undefined))
        )
        .font(.system(size: 10)).foregroundStyle(Lab.secondary)
      } else if failed {
        Text(
          strings(
            "Replicate details are missing or disagree with the check record. The distribution is unavailable."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary)
      } else {
        ProgressView().controlSize(.small)
      }
      Text(
        strings(
          "Each difference uses the simulated and observed matrix on the same posterior tree. The observed statistic can change with the tree. Zero means equality; this is a discrepancy plot, not a convergence test."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted).fixedSize(
        horizontal: false, vertical: true)
    }.task(id: prediction.id) {
      let prediction = prediction
      distributions =
        (try? await Task.detached { try PredictiveDistribution.read(prediction) }.value) ?? []
      failed = distributions.isEmpty
    }
  }
}
