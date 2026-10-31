import Foundation
import Observation

/// The engine's `predictive.json`: paired posterior predictive discrepancies
/// between the observed matrix and matrices simulated on the same sampled tree.
struct PredictiveReport: Decodable, Sendable {
  struct Tail: Decodable, Sendable {
    let validPairs: Int
    let undefinedPairs: Int
    let lowerCount: Int
    let upperCount: Int
    let ties: Int
    let pLower: Double?
    let pUpper: Double?
    let lowerSimulationInterval95: [Double]?
    let upperSimulationInterval95: [Double]?
  }
  struct Source: Decodable, Sendable {
    let directory: String
    let matrixSha256: String
    let totalSamples: Int
    let discardedSamples: Int
    let retainedSamples: Int
    let firstRetainedState: UInt64
    let maxReconstructedLikelihoodDifference: Double
  }
  let schemaVersion: Int
  let method: String
  let status: String
  let seed: UInt64
  let replicates: Int
  let maxAttemptsPerCharacter: Int
  let source: Source
  let global: [String: Tail]
  let convergenceAssessed: Bool
  let createdUnixSeconds: Double
  let wallSeconds: Double

  /// The matrix-wide statistics, in the engine's order.
  static let statistics = [
    "parsimony_length", "consistency_index", "retention_index", "pattern_surprise",
    "informative_characters",
  ]

  static func read(_ directory: URL) throws -> Self {
    let url = directory.appendingPathComponent("predictive.json")
    let info = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard info.isRegularFile == true, (info.fileSize ?? 0) <= 64 * 1024 * 1024 else {
      throw ImportError.message("The predictive check record is invalid.")
    }
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let value = try decoder.decode(Self.self, from: Data(contentsOf: url))
    guard value.schemaVersion == 1, value.status == "completed",
      value.method == "posterior-predictive-paired-v1", value.replicates > 0,
      value.source.retainedSamples > 0, value.source.discardedSamples >= 0,
      value.source.retainedSamples + value.source.discardedSamples == value.source.totalSamples,
      !value.convergenceAssessed,
      statistics.allSatisfy({ name in
        value.global[name].map { $0.validPairs + $0.undefinedPairs == value.replicates } == true
      })
    else { throw ImportError.message("The predictive check record is invalid.") }
    return value
  }
}

struct SavedPrediction: Identifiable, Sendable {
  let directory: URL
  let report: PredictiveReport
  var id: String { directory.resolvingSymlinksInPath().standardizedFileURL.path }
  var createdAt: Date { Date(timeIntervalSince1970: report.createdUnixSeconds) }

  static func read(_ directory: URL) throws -> Self {
    Self(directory: directory, report: try PredictiveReport.read(directory))
  }
}

/// One `rove predict` call on a completed chain.
struct PredictiveRequest: Sendable {
  let source: URL
  let matrix: URL
  let output: URL
  let executable: URL
  let burnin: Int
  let replicates: Int
  let seed: UInt64

  var arguments: [String] {
    [
      "predict", source.path, "--matrix", matrix.path, "--output", output.path,
      "--burnin-percent", String(burnin), "--seed", String(seed),
      "--replicates", String(replicates), "--cancel-on-stdin-close",
    ]
  }

  /// Runs the engine; a cancelled check leaves no output directory.
  func execute(with tool: EngineTool, onProgress: @escaping @Sendable (Int) -> Void)
    async throws -> SavedPrediction
  {
    guard (0..<100).contains(burnin), (1...10_000).contains(replicates) else {
      throw ImportError.message(
        "Choose a burn-in from 0 to 99 percent and 1 to 10,000 replicates.")
    }
    guard FileManager.default.isReadableFile(atPath: matrix.path) else {
      throw ImportError.message("The original matrix is no longer at its recorded path.")
    }
    try FileManager.default.createDirectory(
      at: output.deletingLastPathComponent(), withIntermediateDirectories: true)
    let outcome = try await tool.execute(executable: executable, arguments: arguments) { line in
      // Progress lines read "posterior prediction: done/total".
      let prefix = "posterior prediction: "
      guard line.hasPrefix(prefix), let slash = line.lastIndex(of: "/"),
        let done = Int(line[line.index(line.startIndex, offsetBy: prefix.count)..<slash])
      else { return }
      onProgress(done)
    }
    guard outcome.status == 0 else { throw ImportError.readerFailed(outcome.log) }
    return try SavedPrediction.read(output)
  }
}

@MainActor @Observable
final class PredictiveCheckModel {
  let entry: HistoryEntry
  let executable: URL
  var burnin = "10"
  var replicates = "100"
  var seed: String
  var chain = 1
  private(set) var busy = false
  private(set) var completed = 0
  private(set) var total = 0
  private(set) var predictions: [SavedPrediction] = []
  private(set) var error: String?
  private(set) var details: String?
  private var task: Task<Void, Never>?
  private var worker: EngineTool?

  init(entry: HistoryEntry, executable: URL) {
    self.entry = entry
    self.executable = executable
    seed = entry.draft?.settings.seed ?? "42"
  }

  var chains: Int { entry.draft?.settings.chains ?? 1 }

  /// A prior-only run has no fit to check.
  var available: Bool {
    entry.status == .completed && entry.draft?.settings.sampleFromPrior == false
  }

  var valid: Bool {
    Int(burnin).map { (0..<100).contains($0) } == true
      && Int(replicates).map { (1...10_000).contains($0) } == true && UInt64(seed) != nil
      && (1...chains).contains(chain)
  }

  /// The engine directory to predict from. Groups are not pooled: one chain.
  var source: URL {
    chains > 1
      ? entry.directory.appendingPathComponent(String(format: "results/chain-%02d", chain))
      : RunFiles.output(in: entry.directory)
  }

  func load() async {
    guard let draft = entry.draft else { return }
    let directory = entry.directory.appendingPathComponent("predictions")
    let hash = draft.matrixSHA256
    predictions = await Task.detached {
      let children =
        (try? FileManager.default.contentsOfDirectory(
          at: directory, includingPropertiesForKeys: nil)) ?? []
      return children.compactMap { try? SavedPrediction.read($0) }
        .filter { $0.report.source.matrixSha256 == hash }
        .sorted { $0.createdAt > $1.createdAt }
    }.value
  }

  func start() {
    guard !busy, available, valid, let draft = entry.draft, let percent = Int(burnin),
      let count = Int(replicates), let seed = UInt64(seed)
    else { return }
    let request = PredictiveRequest(
      source: source, matrix: URL(fileURLWithPath: draft.matrixPath),
      output: entry.directory.appendingPathComponent(
        "predictions/prediction-\(UUID().uuidString)"),
      executable: executable, burnin: percent, replicates: count, seed: seed)
    busy = true
    completed = 0
    total = count
    error = nil
    details = nil
    let tool = EngineTool()
    worker = tool
    task = Task {
      do {
        let result = try await request.execute(with: tool) { [weak self] done in
          Task { @MainActor in self?.completed = done }
        }
        predictions.insert(result, at: 0)
      } catch is CancellationError {
        error = "The predictive check was cancelled."
      } catch {
        self.error = "The predictive check could not be completed."
        details = error.localizedDescription
      }
      busy = false
      worker = nil
    }
  }

  func cancel() { worker?.cancel() }
  func waitForExit() async { await task?.value }
}
