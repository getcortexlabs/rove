import AppKit
import Observation

/// A comparison of independent runs, saved beside the run it was started from.
struct SavedComparison: Identifiable, Sendable {
  let file: URL
  let report: ChainComparison
  let createdAt: Date?
  /// The seed of each compared chain, from the engine record in its
  /// directory; nil when the record is unavailable.
  let seeds: [UInt64?]
  var id: String { file.resolvingSymlinksInPath().standardizedFileURL.path }
  var sources: [String] { report.sources?.map(\.directory) ?? [] }

  static func read(_ file: URL) throws -> Self {
    let report = try ChainComparison.read(file)
    guard let sources = report.sources, sources.count == report.chainCount else {
      throw RunFailure.invalidCompletion
    }
    let created = try? file.resourceValues(forKeys: [.creationDateKey]).creationDate
    let seeds = sources.map { source in
      (try? EngineRecord.read(URL(fileURLWithPath: source.directory).appendingPathComponent("run.json")))?
        .settings.seed
    }
    return Self(file: file, report: report, createdAt: created, seeds: seeds)
  }
}

/// Runs `rove compare` on chain directories; cancellation stops the engine.
final class RunComparisonProcess: @unchecked Sendable {
  private let lock = NSLock()
  private var child: Process?
  private var cancelled = false

  func cancel() {
    lock.lock()
    defer { lock.unlock() }
    cancelled = true
    if child?.isRunning == true { child?.terminate() }
  }

  func execute(directories: [URL], executable: URL, burnin: Int, output: URL) async throws
    -> SavedComparison
  {
    try await withCheckedThrowingContinuation { continuation in
      Thread.detachNewThread {
        autoreleasepool {
          continuation.resume(
            with: Result {
              try self.executeBlocking(
                directories: directories, executable: executable, burnin: burnin, output: output)
            })
        }
      }
    }
  }

  private func executeBlocking(directories: [URL], executable: URL, burnin: Int, output: URL)
    throws -> SavedComparison
  {
    guard (2...16).contains(directories.count) else {
      throw ImportError.message("Choose runs with 2 to 16 chains in total.")
    }
    guard (0..<100).contains(burnin) else {
      throw ImportError.message("Choose a burn-in from 0 to 99 percent.")
    }
    let manager = FileManager.default
    try manager.createDirectory(
      at: output.deletingLastPathComponent(), withIntermediateDirectories: true)
    let partial = output.appendingPathExtension("partial")
    let log = manager.temporaryDirectory.appendingPathComponent("rove-compare-\(UUID()).log")
    manager.createFile(atPath: partial.path, contents: nil)
    manager.createFile(atPath: log.path, contents: nil)
    defer {
      try? manager.removeItem(at: log)
      try? manager.removeItem(at: partial)
    }
    let result = try FileHandle(forWritingTo: partial)
    let errors = try FileHandle(forWritingTo: log)
    defer {
      try? result.close()
      try? errors.close()
    }
    let process = Process()
    process.executableURL = executable
    process.arguments =
      ["compare"] + directories.map(\.path) + ["--burnin-percent", String(burnin)]
    process.standardInput = FileHandle.nullDevice
    process.standardOutput = result
    process.standardError = errors
    lock.lock()
    if cancelled {
      lock.unlock()
      throw CancellationError()
    }
    do { try process.run() } catch {
      lock.unlock()
      throw error
    }
    child = process
    lock.unlock()
    process.waitUntilExit()
    lock.lock()
    child = nil
    let stopped = cancelled
    lock.unlock()
    if stopped { throw CancellationError() }
    guard process.terminationStatus == 0 else {
      let handle = try FileHandle(forReadingFrom: log)
      defer { try? handle.close() }
      let data = try handle.read(upToCount: 16_384) ?? Data()
      throw ImportError.readerFailed(
        String(decoding: data, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines))
    }
    try result.close()
    _ = try ChainComparison.read(partial)
    try manager.moveItem(at: partial, to: output)
    return try SavedComparison.read(output)
  }
}

@MainActor @Observable
final class RunComparisonModel {
  let entry: HistoryEntry
  let executable: URL
  var burnin = "10"
  var selected: Set<String> = []
  private(set) var busy = false
  private(set) var comparisons: [SavedComparison] = []
  private(set) var error: String?
  private(set) var details: String?
  private var task: Task<Void, Never>?
  private var worker: RunComparisonProcess?

  init(entry: HistoryEntry, executable: URL) {
    self.entry = entry
    self.executable = executable
  }

  var valid: Bool { Int(burnin).map { (0..<100).contains($0) } == true }

  /// Completed runs that `rove compare` accepts next to this one: the same
  /// matrix, model, constraints and sampling interval, with seeds that do not
  /// overlap. Steps may differ; the comparison uses the common prefix.
  static func compatible(_ a: HistoryEntry, _ b: HistoryEntry) -> Bool {
    guard a.id != b.id, a.status == .completed, b.status == .completed,
      let x = a.draft, let y = b.draft, x.matrixSHA256 == y.matrixSHA256,
      x.constraints == y.constraints, x.settings.clock == y.settings.clock,
      x.settings.conditioning == y.settings.conditioning,
      x.settings.partitioning == y.settings.partitioning,
      x.settings.gapsAsMissing == y.settings.gapsAsMissing,
      x.settings.dating == y.settings.dating, x.settings.sampleEvery == y.settings.sampleEvery,
      let p = UInt64(x.settings.seed), let q = UInt64(y.settings.seed)
    else { return false }
    let first = p...(p &+ UInt64(max(x.settings.chains - 1, 0)))
    let second = q...(q &+ UInt64(max(y.settings.chains - 1, 0)))
    return !first.overlaps(second)
  }

  func candidates(in entries: [HistoryEntry]) -> [HistoryEntry] {
    entries.filter { Self.compatible(entry, $0) }
  }

  static let maximumChains = 16

  /// The seeds a run's chains use: the recorded seed and one more per chain.
  static func seeds(_ entry: HistoryEntry) -> ClosedRange<UInt64>? {
    guard let draft = entry.draft, let base = UInt64(draft.settings.seed) else { return nil }
    return base...(base &+ UInt64(max(draft.settings.chains - 1, 0)))
  }

  private static func chains(_ entry: HistoryEntry) -> Int {
    max(entry.draft?.settings.chains ?? 1, 1)
  }

  /// Whether `candidate` can join the current selection: its seeds must not
  /// overlap any selected run and the total must stay within the engine's limit.
  func canSelect(_ candidate: HistoryEntry, in candidates: [HistoryEntry]) -> Bool {
    if selected.contains(candidate.id) { return true }
    guard let range = Self.seeds(candidate) else { return false }
    let chosen = candidates.filter { selected.contains($0.id) }
    let taken = ([entry] + chosen).compactMap(Self.seeds)
    let total = ([entry] + chosen).map(Self.chains).reduce(0, +)
    return !taken.contains { $0.overlaps(range) }
      && total + Self.chains(candidate) <= Self.maximumChains
  }

  /// The engine directories holding one chain each.
  static func chainDirectories(_ entry: HistoryEntry) -> [URL] {
    let chains = entry.draft?.settings.chains ?? 1
    guard chains > 1 else { return [RunFiles.output(in: entry.directory)] }
    return (1...chains).map {
      entry.directory.appendingPathComponent(String(format: "results/chain-%02d", $0))
    }
  }

  func load() async {
    let directory = entry.directory.appendingPathComponent("comparisons")
    let values = await Task.detached {
      let children =
        (try? FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil))
        ?? []
      return children.filter { $0.pathExtension == "json" }
        .compactMap { try? SavedComparison.read($0) }
        .sorted { ($0.createdAt ?? .distantPast) > ($1.createdAt ?? .distantPast) }
    }.value
    comparisons = values
  }

  func start(with others: [HistoryEntry]) {
    guard !busy, valid, let percent = Int(burnin) else { return }
    let chosen = others.filter { selected.contains($0.id) && Self.compatible(entry, $0) }
    let ranges = ([entry] + chosen).compactMap(Self.seeds)
    let directories = ([entry] + chosen).flatMap(Self.chainDirectories)
    guard !chosen.isEmpty, ranges.count == chosen.count + 1,
      (2...Self.maximumChains).contains(directories.count),
      ranges.indices.allSatisfy({ i in ranges.indices.allSatisfy { $0 == i || !ranges[$0].overlaps(ranges[i]) } })
    else { return }
    let output = entry.directory.appendingPathComponent(
      "comparisons/comparison-\(UUID().uuidString).json")
    busy = true
    error = nil
    details = nil
    let process = RunComparisonProcess()
    worker = process
    let executable = executable
    task = Task {
      do {
        let result = try await process.execute(
          directories: directories, executable: executable, burnin: percent, output: output)
        comparisons.insert(result, at: 0)
      } catch is CancellationError {
        error = "The comparison was cancelled."
      } catch {
        self.error = "The runs could not be compared."
        details = error.localizedDescription
      }
      busy = false
      worker = nil
    }
  }

  func cancel() { worker?.cancel() }
  func waitForExit() async { await task?.value }
}
