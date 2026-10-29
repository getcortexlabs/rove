import AppKit
import Observation

struct TracePoint: Decodable, Sendable, Identifiable {
  let state: UInt64
  let value: Double
  let retained: Bool
  var id: UInt64 { state }
}

struct TraceStatistic: Decodable, Sendable, Identifiable {
  let name: String
  let mean: Double
  let sd: Double
  let interval: [Double]?
  let ess: Double?
  let essReason: String?
  let lastLag: Int
  let lagLimitReached: Bool
  let points: [TracePoint]
  var id: String { name }
}

struct TraceReport: Decodable, Sendable {
  struct Source: Decodable, Sendable {
    let path: String
    let sha256: String
    let bytes: Int
    let completion: String
  }
  struct Burnin: Decodable, Sendable {
    let unit: String
    let value: Int
  }
  let schemaVersion: Int
  let kind: String
  let createdUnixSeconds: Double
  let source: Source
  let burnin: Burnin
  let totalSamples: Int
  let discardedSamples: Int
  let retainedSamples: Int
  let firstState: UInt64
  let firstRetainedState: UInt64
  let lastState: UInt64
  let sampleEvery: UInt64?
  let essMethod: String
  let intervalMethod: String
  let statisticsUseAllRetainedSamples: Bool
  let convergenceAssessed: Bool
  let topologyAssessed: Bool
  let rhat: Double?
  let columns: [TraceStatistic]

  static func read(_ directory: URL) throws -> Self {
    let path = directory.appendingPathComponent("diagnostics.json")
    let info = try path.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard info.isRegularFile == true, (info.fileSize ?? 0) <= 16 * 1024 * 1024 else {
      throw ImportError.message("The diagnostic record is missing or too large.")
    }
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let value = try decoder.decode(Self.self, from: Data(contentsOf: path))
    guard value.schemaVersion == 1, value.kind == "within_chain_diagnostics",
      value.burnin.unit == "percent", (0..<100).contains(value.burnin.value),
      (1...1_000_000).contains(value.totalSamples), value.discardedSamples >= 0,
      value.retainedSamples > 0,
      value.totalSamples - value.discardedSamples == value.retainedSamples,
      value.discardedSamples == value.totalSamples * value.burnin.value / 100,
      value.firstState <= value.firstRetainedState, value.firstRetainedState <= value.lastState,
      value.statisticsUseAllRetainedSamples, !value.convergenceAssessed, !value.topologyAssessed,
      value.rhat == nil, value.source.sha256.count == 64, value.createdUnixSeconds.isFinite,
      (1...128).contains(value.columns.count),
      Set(value.columns.map(\.name)).count == value.columns.count
    else { throw ImportError.message("The diagnostic record is invalid.") }
    for column in value.columns {
      guard column.mean.isFinite, column.sd.isFinite, column.sd >= 0,
        column.ess.map({ $0.isFinite && $0 > 0 && $0 <= Double(value.retainedSamples) }) ?? true,
        !column.points.isEmpty, column.points.count <= 1028,
        column.points.first?.state == value.firstState,
        column.points.last?.state == value.lastState,
        column.points.allSatisfy({
          $0.value.isFinite && $0.retained == ($0.state >= value.firstRetainedState)
        }),
        zip(column.points, column.points.dropFirst()).allSatisfy({ $0.state < $1.state })
      else { throw ImportError.message("The diagnostic record is invalid.") }
      if let range = column.interval {
        guard range.count == 2, range.allSatisfy(\.isFinite), range[0] <= range[1] else {
          throw ImportError.message("The diagnostic record is invalid.")
        }
      }
    }
    return value
  }
}

struct SavedDiagnostics: Sendable {
  let directory: URL
  let report: TraceReport
}

final class TraceDiagnosticsProcess: @unchecked Sendable {
  private let lock = NSLock()
  private var input: FileHandle?
  private var cancelled = false

  func cancel() {
    lock.lock()
    defer { lock.unlock() }
    cancelled = true
    try? input?.close()
    input = nil
  }

  func execute(directory: URL, executable: URL, burnin: Int) throws -> SavedDiagnostics {
    guard (0..<100).contains(burnin) else {
      throw ImportError.message("Choose a burn-in from 0 to 99 percent.")
    }
    let parent = directory.appendingPathComponent("diagnostics")
    try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
    let stamp = Int64(Date().timeIntervalSince1970 * 1000)
    let output = parent.appendingPathComponent("trace-\(stamp)-\(UUID())")
    let log = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-diagnostics-\(UUID()).log")
    FileManager.default.createFile(atPath: log.path, contents: nil)
    defer { try? FileManager.default.removeItem(at: log) }
    let errors = try FileHandle(forWritingTo: log)
    defer { try? errors.close() }
    let pipe = Pipe()
    let child = Process()
    child.executableURL = executable
    child.arguments = [
      "diagnose", directory.appendingPathComponent("results/trace.log").path,
      "--burnin-percent", String(burnin), "--output", output.path, "--cancel-on-stdin-close",
    ]
    child.standardInput = pipe
    child.standardOutput = FileHandle.nullDevice
    child.standardError = errors
    lock.lock()
    if cancelled {
      lock.unlock()
      throw CancellationError()
    }
    do { try child.run() } catch {
      lock.unlock()
      throw error
    }
    input = pipe.fileHandleForWriting
    lock.unlock()
    try? pipe.fileHandleForReading.close()
    child.waitUntilExit()
    lock.lock()
    try? input?.close()
    input = nil
    let stopped = cancelled
    lock.unlock()
    if stopped { throw CancellationError() }
    guard child.terminationStatus == 0 else {
      let handle = try FileHandle(forReadingFrom: log)
      defer { try? handle.close() }
      throw ImportError.readerFailed(
        String(decoding: try handle.read(upToCount: 16384) ?? Data(), as: UTF8.self))
    }
    return SavedDiagnostics(directory: output, report: try TraceReport.read(output))
  }
}

@MainActor @Observable
final class TraceDiagnosticsModel: Identifiable {
  let id = UUID()
  let directory: URL
  let executable: URL
  var burnin = "10"
  var selected = "posterior"
  var query = ""
  private(set) var result: SavedDiagnostics?
  private(set) var busy = false
  private(set) var error: String?
  private(set) var details: String?
  private var loaded = false
  private var task: Task<Void, Never>?
  private var worker: TraceDiagnosticsProcess?
  var valid: Bool { Int(burnin).map { (0..<100).contains($0) } == true }
  var statistic: TraceStatistic? { result?.report.columns.first { $0.name == selected } }
  var columns: [TraceStatistic] {
    let priority = ["posterior", "likelihood", "prior"]
    return (result?.report.columns ?? []).filter {
      query.isEmpty || $0.name.localizedStandardContains(query)
    }
    .sorted { a, b in
      let x = priority.firstIndex(of: a.name) ?? 3
      let y = priority.firstIndex(of: b.name) ?? 3
      return x == y ? a.name < b.name : x < y
    }
  }

  init(directory: URL, executable: URL) {
    self.directory = directory
    self.executable = executable
  }

  func load() async {
    guard !loaded else { return }
    loaded = true
    let directory = directory
    let previous = await Task.detached {
      let folders =
        (try? FileManager.default.contentsOfDirectory(
          at: directory.appendingPathComponent("diagnostics"), includingPropertiesForKeys: nil))
        ?? []
      for folder in folders.filter({ $0.lastPathComponent.hasPrefix("trace-") }).sorted(by: {
        $0.lastPathComponent > $1.lastPathComponent
      }).prefix(20) {
        if let record = try? TraceReport.read(folder) { return record.burnin.value }
      }
      return 10
    }.value
    guard !Task.isCancelled else { return }
    burnin = String(previous)
    start()
    await waitForExit()
  }

  func start() {
    guard !busy, valid, let percent = Int(burnin) else { return }
    busy = true
    error = nil
    details = nil
    let process = TraceDiagnosticsProcess()
    worker = process
    let directory = directory
    let executable = executable
    task = Task {
      do {
        let saved = try await Task.detached {
          try process.execute(directory: directory, executable: executable, burnin: percent)
        }.value
        result = saved
        if statistic == nil { selected = saved.report.columns.first?.name ?? "posterior" }
      } catch is CancellationError {
        error = "Diagnostic calculation was cancelled."
      } catch {
        self.error = "The trace could not be analyzed."
        details = error.localizedDescription
      }
      busy = false
      worker = nil
    }
  }
  func cancel() { worker?.cancel() }
  func waitForExit() async { await task?.value }
}
