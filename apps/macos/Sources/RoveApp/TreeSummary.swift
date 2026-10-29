import AppKit
import Observation

struct SummaryRecord: Decodable, Sendable {
  let schemaVersion: Int
  let status: String
  let method: String
  let totalSamples: Int
  let discardedSamples: Int
  let retainedSamples: Int
  let selectedState: UInt64
  let heightPolicy: String
  let annotatedTree: String
  let rawTrees: String
  let convergenceAssessed: Bool

  static func read(_ directory: URL) throws -> Self {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let value = try decoder.decode(
      Self.self, from: RecordFile.read(directory.appendingPathComponent("summary.json")))
    guard value.schemaVersion == 1, value.status == "completed", value.method == "MCC",
      value.totalSamples > 0, value.discardedSamples >= 0, value.retainedSamples > 0,
      value.discardedSamples <= value.totalSamples,
      value.retainedSamples == value.totalSamples - value.discardedSamples,
      ["keep", "ca"].contains(value.heightPolicy), value.annotatedTree == "mcc.tree",
      value.rawTrees == "samples.trees", !value.convergenceAssessed
    else {
      throw ImportError.message("The tree summary record is invalid.")
    }
    for name in [value.annotatedTree, value.rawTrees] {
      let info = try directory.appendingPathComponent(name).resourceValues(forKeys: [
        .isRegularFileKey, .fileSizeKey,
      ])
      guard info.isRegularFile == true, (info.fileSize ?? 0) > 0 else {
        throw ImportError.message("The tree summary files are missing.")
      }
    }
    return value
  }
}

struct SavedSummary: Identifiable, Sendable {
  let directory: URL
  let record: SummaryRecord
  var id: String { directory.resolvingSymlinksInPath().standardizedFileURL.path }
}

final class TreeSummaryProcess: @unchecked Sendable {
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

  func execute(directory: URL, executable: URL, burnin: Int, heights: String) async throws
    -> SavedSummary
  {
    try await withCheckedThrowingContinuation { continuation in
      Thread.detachNewThread {
        autoreleasepool {
          continuation.resume(
            with: Result {
              try self.executeBlocking(
                directory: directory, executable: executable, burnin: burnin, heights: heights)
            })
        }
      }
    }
  }

  private func executeBlocking(directory: URL, executable: URL, burnin: Int, heights: String)
    throws -> SavedSummary
  {
    guard (0..<100).contains(burnin), ["keep", "ca"].contains(heights) else {
      throw ImportError.message("Choose a burn-in from 0 to 99 percent.")
    }
    guard HistoryEntry.read(RunFiles.session(in: directory)).status == .completed else {
      throw ImportError.message("Only completed runs can be summarized.")
    }
    let parent = directory.appendingPathComponent("summaries")
    try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
    let output = parent.appendingPathComponent("summary-\(UUID().uuidString)")
    let log = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-summary-\(UUID()).log")
    FileManager.default.createFile(atPath: log.path, contents: nil)
    defer { try? FileManager.default.removeItem(at: log) }
    let errors = try FileHandle(forWritingTo: log)
    defer { try? errors.close() }
    let pipe = Pipe()
    let child = Process()
    child.executableURL = executable
    child.arguments = [
      "summarize", RunFiles.output(in: directory).appendingPathComponent("trees.nexus").path,
      "--output", output.path, "--burnin-percent", String(burnin), "--heights", heights,
      "--cancel-on-stdin-close",
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
      let data = try handle.read(upToCount: 16_384) ?? Data()
      throw ImportError.readerFailed(String(decoding: data, as: UTF8.self))
    }
    return SavedSummary(directory: output, record: try SummaryRecord.read(output))
  }
}

@MainActor @Observable
final class TreeSummaryModel: Identifiable {
  let id = UUID()
  let directory: URL
  let executable: URL
  var burnin = "10"
  var heights = "ca"
  private(set) var busy = false
  private(set) var summaries: [SavedSummary] = []
  private(set) var error: String?
  private(set) var details: String?
  private var task: Task<Void, Never>?
  private var worker: TreeSummaryProcess?
  var valid: Bool { Int(burnin).map { (0..<100).contains($0) } == true }

  init(directory: URL, executable: URL) {
    self.directory = directory
    self.executable = executable
  }

  func load() async {
    let directory = directory
    let values = await Task.detached {
      let children =
        (try? FileManager.default.contentsOfDirectory(
          at: directory.appendingPathComponent("summaries"),
          includingPropertiesForKeys: nil)) ?? []
      return children.sorted { $0.lastPathComponent < $1.lastPathComponent }.compactMap { url in
        (try? SummaryRecord.read(url)).map { SavedSummary(directory: url, record: $0) }
      }
    }.value
    let existing = Set(summaries.map(\.id))
    summaries.insert(contentsOf: values.filter { !existing.contains($0.id) }, at: 0)
  }

  func start() {
    guard !busy, valid, let percent = Int(burnin) else { return }
    busy = true
    error = nil
    details = nil
    let process = TreeSummaryProcess()
    worker = process
    let directory = directory
    let executable = executable
    let heights = heights
    task = Task {
      do {
        let result = try await process.execute(
          directory: directory, executable: executable, burnin: percent, heights: heights)
        summaries.append(result)
      } catch is CancellationError {
        error = "Tree summarization was cancelled."
      } catch {
        self.error = "The trees could not be summarized."
        details = error.localizedDescription
      }
      busy = false
      worker = nil
    }
  }

  func cancel() { worker?.cancel() }
  func waitForExit() async { await task?.value }
}
