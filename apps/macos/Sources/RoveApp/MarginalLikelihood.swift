import Foundation
import Observation

/// The engine's `marginal.json`: stepping-stone and path-sampling estimates
/// of the log marginal likelihood from a schedule of power posteriors.
struct MarginalEstimate: Decodable, Sendable {
  struct Estimate: Decodable, Sendable {
    let logMarginalLikelihood: Double
    let standardError: Double?
  }
  struct Power: Decodable, Sendable, Identifiable {
    let index: Int
    let power: Double
    let samples: Int
    let meanLogLikelihood: Double
    let sdLogLikelihood: Double
    let ess: Double?
    let lagLimitReached: Bool
    var id: Int { index }
  }
  struct Schedule: Decodable, Sendable {
    let seed: UInt64
    let stepsPerPower: Int
    let sampleEvery: Int
    let burninPercent: Int
    let powers: Int
    let alpha: Double
  }
  let kind: String
  let steppingStone: Estimate
  let pathSampling: Estimate
  let powers: [Power]
  let settings: Schedule
  let inputSha256: String

  static func read(_ directory: URL) throws -> Self {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let value = try decoder.decode(
      Self.self, from: RecordFile.read(directory.appendingPathComponent("marginal.json")))
    guard value.kind == "marginal_likelihood", value.powers.count == value.settings.powers,
      value.powers.count >= 2,
      [value.steppingStone, value.pathSampling].allSatisfy({
        $0.logMarginalLikelihood.isFinite && $0.standardError.map { $0.isFinite && $0 >= 0 } ?? true
      })
    else { throw ImportError.message("The marginal likelihood record is invalid.") }
    return value
  }
}

/// The `run.json` of a marginal-likelihood run: the model and the schedule.
struct MarginalRunRecord: Decodable, Sendable {
  struct Settings: Decodable, Sendable {
    let seed: UInt64
    let stepsPerPower: UInt64
    let sampleEvery: UInt64
    var tuning: Bool? = nil
  }
  let status: String
  let kind: String
  let completedSteps: UInt64
  let input: EngineRecord.Input
  let model: EngineRecord.Model
  let settings: Settings
  var moves: [EngineRecord.MoveRecord]? = nil
  let startedUnixSeconds: Double?
  let wallSeconds: Double?

  static func read(_ url: URL) throws -> Self {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    return try decoder.decode(Self.self, from: RecordFile.read(url))
  }
}

/// A completed estimate saved under a run's `marginal` folder.
struct SavedEstimate: Identifiable, Sendable {
  let directory: URL
  let run: URL
  let estimate: MarginalEstimate
  let record: MarginalRunRecord
  var id: String { directory.resolvingSymlinksInPath().standardizedFileURL.path }
  var createdAt: Date? { record.startedUnixSeconds.map { Date(timeIntervalSince1970: $0) } }

  static func read(_ directory: URL, run: URL) throws -> Self {
    let record = try MarginalRunRecord.read(directory.appendingPathComponent("run.json"))
    guard record.status == "completed", record.kind == "marginal_likelihood_run" else {
      throw RunFailure.invalidCompletion
    }
    let estimate = try MarginalEstimate.read(directory)
    guard estimate.inputSha256 == record.input.sha256 else { throw RunFailure.invalidCompletion }
    return Self(directory: directory, run: run, estimate: estimate, record: record)
  }

  /// Whether the estimate is for the model, constraints and proposal
  /// schedule of `draft`, as `EngineRecord.matches` checks a sampling run.
  func matches(_ draft: AnalysisDraft) -> Bool {
    let settings = draft.settings
    let dating: EngineDatingModel?
    do { dating = try settings.dating.model() } catch { return false }
    return record.input.sha256 == draft.matrixSHA256
      && record.input.gaps == (settings.gapsAsMissing ? "missing" : "reject")
      && record.model.monophyly == draft.constraints && record.model.preset == settings.clock
      && record.model.conditioning
        == EngineRecord.Model.conditioningDescriptions[settings.conditioning]
      && (record.model.partitioning ?? "declared") == settings.partitioning
      && record.model.dating == dating
      && record.settings.sampleEvery == UInt64(settings.sampleEvery)
      && (record.settings.tuning ?? true) == settings.tuning
      && settings.weights.allSatisfy { override in
        record.moves?.first { $0.name == override.move }?.weight == override.value
      }
      && settings.tuningValues.allSatisfy { override in
        record.moves?.first { $0.name == override.move }?.initialTuning == override.value
      }
  }

  /// Completed estimates of a History run that match its setup, newest first.
  static func list(in entry: HistoryEntry) -> [SavedEstimate] {
    guard let draft = entry.draft else { return [] }
    let folder = entry.directory.appendingPathComponent("marginal")
    let children =
      (try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil))
      ?? []
    return children.compactMap { try? read($0, run: entry.directory) }.filter { $0.matches(draft) }
      .sorted { ($0.createdAt ?? .distantPast) > ($1.createdAt ?? .distantPast) }
  }
}

/// The power-posterior schedule of one estimate.
struct MarginalSchedule: Sendable {
  let powers: Int
  let alpha: Double
  let steps: Int
  let sampleEvery: Int
  let burnin: Int
  let seed: UInt64

  var valid: Bool {
    (2...256).contains(powers) && alpha.isFinite && alpha > 0 && steps > 0 && sampleEvery > 0
      && steps % sampleEvery == 0 && (0..<100).contains(burnin)
  }

  func arguments(draft: AnalysisDraft, output: URL) throws -> [String] {
    try [
      "marginal", draft.matrixPath, "--output", output.path, "--seed", String(seed),
      "--steps", String(steps), "--sample-every", String(sampleEvery),
      "--powers", String(powers), "--alpha", String(alpha), "--burnin-percent", String(burnin),
    ] + draft.settings.modelArguments(monophyly: draft.constraints)
      + ["--cancel-on-stdin-close", "--expect-input-sha256", draft.matrixSHA256]
  }
}

/// Where a running estimate is: powers started so far and the step within the current one.
struct MarginalProgress: Sendable, Equatable {
  var powersStarted = 0
  var step = 0
}

@MainActor @Observable
final class MarginalLikelihoodModel {
  let entry: HistoryEntry
  let executable: URL
  var powers = "32"
  var alpha = "0.3"
  var steps: String
  var burnin = "20"
  var seed: String
  private(set) var busy = false
  private(set) var progress = MarginalProgress()
  private(set) var estimates: [SavedEstimate] = []
  private(set) var others: [SavedEstimate] = []
  private(set) var error: String?
  private(set) var details: String?
  private var task: Task<Void, Never>?
  private var worker: EngineTool?
  private var currentPower: String?
  private var lastStep = 0

  init(entry: HistoryEntry, executable: URL) {
    self.entry = entry
    self.executable = executable
    let settings = entry.draft?.settings ?? AnalysisSettings()
    seed = settings.seed
    // A tenth of the run per power, as a multiple of its sampling interval.
    let interval = max(settings.sampleEvery, 1)
    steps = String(max(settings.steps / 10 / interval, 1) * interval)
  }

  var sampleEvery: Int { entry.draft?.settings.sampleEvery ?? 1 }

  /// Estimated FBD parameters add observation factors whose normalization is
  /// not derived for model comparison, so no estimate is offered for them.
  /// Prior-only runs stand for their priors, not for a fitted model.
  var available: Bool {
    entry.status == .completed
      && entry.draft.map { !$0.settings.dating.estimatesProcess && !$0.settings.sampleFromPrior }
        == true
  }

  var schedule: MarginalSchedule? {
    guard let powers = Int(powers), let alpha = Double(alpha), let steps = Int(steps),
      let burnin = Int(burnin), let seed = UInt64(seed)
    else { return nil }
    let value = MarginalSchedule(
      powers: powers, alpha: alpha, steps: steps, sampleEvery: sampleEvery, burnin: burnin,
      seed: seed)
    return value.valid ? value : nil
  }

  var valid: Bool { schedule != nil }

  /// Runs whose estimates can be compared with this run's: the same matrix,
  /// constraints and gap policy, and either no dating or identical dating
  /// with a fixed process.
  static func comparable(_ a: HistoryEntry, _ b: HistoryEntry) -> Bool {
    guard a.id != b.id, a.status == .completed, b.status == .completed,
      let x = a.draft, let y = b.draft, x.matrixSHA256 == y.matrixSHA256,
      x.constraints == y.constraints, x.settings.gapsAsMissing == y.settings.gapsAsMissing,
      !x.settings.sampleFromPrior, !y.settings.sampleFromPrior
    else { return false }
    if !x.settings.dating.enabled && !y.settings.dating.enabled { return true }
    return x.settings.dating == y.settings.dating && !x.settings.dating.estimatesProcess
  }

  /// The log Bayes factor of `a` over `b` and its standard error, combining
  /// both errors in quadrature.
  static func difference(_ a: MarginalEstimate.Estimate, _ b: MarginalEstimate.Estimate) -> (
    value: Double, standardError: Double?
  ) {
    let error = a.standardError.flatMap { x in
      b.standardError.map { (x * x + $0 * $0).squareRoot() }
    }
    return (a.logMarginalLikelihood - b.logMarginalLikelihood, error)
  }

  func load(history entries: [HistoryEntry]) async {
    let entry = entry
    let related = entries.filter { Self.comparable(entry, $0) }
    let (own, other) = await Task.detached {
      (SavedEstimate.list(in: entry), related.flatMap(SavedEstimate.list))
    }.value
    estimates = own
    others = other
  }

  func start() {
    guard !busy, available, let draft = entry.draft, let schedule else { return }
    let output = entry.directory.appendingPathComponent("marginal/estimate-\(UUID().uuidString)")
    let executable = executable
    let run = entry.directory
    busy = true
    progress = MarginalProgress()
    currentPower = nil
    lastStep = 0
    error = nil
    details = nil
    let tool = EngineTool()
    worker = tool
    task = Task {
      do {
        let arguments = try schedule.arguments(draft: draft, output: output)
        let outcome = try await tool.execute(executable: executable, arguments: arguments) {
          [weak self] line in
          guard let update = Self.progress(from: line) else { return }
          Task { @MainActor in
            guard let self else { return }
            // Powers below 5e-7 print alike, so a step count that restarts
            // also marks the next power.
            if self.currentPower != update.power || update.step <= self.lastStep {
              self.currentPower = update.power
              self.progress.powersStarted += 1
            }
            self.lastStep = update.step
            self.progress.step = update.step
          }
        }
        guard outcome.status == 0 else { throw ImportError.readerFailed(outcome.log) }
        estimates.insert(try SavedEstimate.read(output, run: run), at: 0)
      } catch is CancellationError {
        error = "The estimate was cancelled. Its partial traces stay in the run folder."
      } catch {
        self.error = "The marginal likelihood could not be estimated."
        details = error.localizedDescription
      }
      busy = false
      worker = nil
    }
  }

  /// Parses a progress line: power, step, posterior, likelihood, prior and
  /// seconds separated by tabs. The power changes when the next one starts.
  nonisolated static func progress(from line: String) -> (power: String, step: Int)? {
    let fields = line.split(separator: "\t", omittingEmptySubsequences: false)
    guard fields.count == 6, Double(fields[0]) != nil, let step = Int(fields[1]) else {
      return nil
    }
    return (String(fields[0]), step)
  }

  func cancel() { worker?.cancel() }
  func waitForExit() async { await task?.value }
}
