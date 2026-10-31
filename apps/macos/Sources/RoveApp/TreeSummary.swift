import AppKit
import CryptoKit
import Observation

/// Where a summary's trees come from: the run that holds the summaries, or a
/// tree file written by another program.
enum TreeSummarySource: Sendable, Equatable {
  case run
  case file(URL)
}

/// The record Rove keeps beside the summaries of a tree file it did not write.
struct TreeSource: Codable, Sendable, Equatable {
  let formatVersion: Int
  let name: String
  let path: String
  let sha256: String
  let createdAt: Date

  static let fileName = "tree-source.json"

  static func read(_ url: URL) throws -> Self {
    let decoder = JSONDecoder()
    decoder.dateDecodingStrategy = .iso8601
    let value = try decoder.decode(Self.self, from: RecordFile.read(url))
    guard value.formatVersion == 1, value.sha256.count == 64, !value.name.isEmpty else {
      throw RunFailure.invalidCompletion
    }
    return value
  }

  /// The folder under `parent` that holds the file's summaries: one per file
  /// name and content, so summarizing the same file again lands in the same
  /// place. The record follows the file when it is opened from a new path.
  static func prepare(_ file: URL, under parent: URL) throws -> URL {
    let values = try file.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard values.isRegularFile == true else {
      throw ImportError.message("Choose a tree file, rather than a folder.")
    }
    guard (values.fileSize ?? 0) <= 1024 * 1024 * 1024 else {
      throw ImportError.message("Tree files up to 1 GB can be summarized.")
    }
    let sha256 = try fingerprint(file)
    let stem = file.deletingPathExtension().lastPathComponent.prefix(64)
    let container = parent.appendingPathComponent("trees-\(stem)-\(sha256.prefix(8))")
    let record = container.appendingPathComponent(fileName)
    try FileManager.default.createDirectory(at: container, withIntermediateDirectories: true)
    let existing = try? read(record)
    if existing?.path != file.path {
      let encoder = JSONEncoder()
      encoder.dateEncodingStrategy = .iso8601
      encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
      try encoder.encode(
        Self(
          formatVersion: 1, name: file.lastPathComponent, path: file.path, sha256: sha256,
          createdAt: existing?.createdAt ?? Date())
      ).write(to: record, options: .atomic)
    }
    return container
  }

  /// The SHA-256 of a file, read in pieces.
  static func fingerprint(_ url: URL) throws -> String {
    let handle = try FileHandle(forReadingFrom: url)
    defer { try? handle.close() }
    var hasher = SHA256()
    while let chunk = try handle.read(upToCount: 4 * 1024 * 1024), !chunk.isEmpty {
      hasher.update(data: chunk)
    }
    return hasher.finalize().map { String(format: "%02x", $0) }.joined()
  }
}

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

  /// The readable summaries under a run or tree-file folder, oldest first.
  static func list(in directory: URL) -> [SavedSummary] {
    let children =
      (try? FileManager.default.contentsOfDirectory(
        at: directory.appendingPathComponent("summaries"), includingPropertiesForKeys: nil)) ?? []
    return children.sorted { $0.lastPathComponent < $1.lastPathComponent }.compactMap { url in
      (try? SummaryRecord.read(url)).map { SavedSummary(directory: url, record: $0) }
    }
  }
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

  func execute(
    directory: URL, executable: URL, burnin: Int, unit: String = "percent", heights: String,
    source: TreeSummarySource = .run
  ) async throws -> SavedSummary {
    try await withCheckedThrowingContinuation { continuation in
      Thread.detachNewThread {
        autoreleasepool {
          continuation.resume(
            with: Result {
              try self.executeBlocking(
                directory: directory, executable: executable, burnin: burnin, unit: unit,
                heights: heights, source: source)
            })
        }
      }
    }
  }

  private func executeBlocking(
    directory: URL, executable: URL, burnin: Int, unit: String, heights: String,
    source: TreeSummarySource
  ) throws -> SavedSummary {
    guard ["percent", "samples"].contains(unit), burnin >= 0,
      unit == "samples" || burnin < 100, ["keep", "ca"].contains(heights)
    else {
      throw ImportError.message("Choose a burn-in from 0 to 99 percent or a sample count.")
    }
    let trees: URL
    switch source {
    case .run:
      guard HistoryEntry.read(RunFiles.session(in: directory)).status == .completed else {
        throw ImportError.message("Only completed runs can be summarized.")
      }
      trees = RunFiles.output(in: directory).appendingPathComponent("trees.nexus")
    case .file(let file):
      guard FileManager.default.isReadableFile(atPath: file.path) else {
        throw ImportError.message("The tree file is no longer at its recorded path.")
      }
      trees = file
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
      "summarize", trees.path, "--output", output.path, "--burnin-\(unit)", String(burnin),
      "--heights", heights, "--cancel-on-stdin-close",
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
  let source: TreeSummarySource
  var burnin = "10"
  var burninUnit = "percent"
  var heights = "ca"
  private(set) var busy = false
  private(set) var summaries: [SavedSummary] = []
  private(set) var error: String?
  private(set) var details: String?
  private var task: Task<Void, Never>?
  private var worker: TreeSummaryProcess?
  var valid: Bool {
    guard let value = Int(burnin), value >= 0 else { return false }
    return burninUnit == "samples" || value < 100
  }

  init(directory: URL, executable: URL, source: TreeSummarySource = .run) {
    self.directory = directory
    self.executable = executable
    self.source = source
  }

  func load() async {
    let directory = directory
    let values = await Task.detached { SavedSummary.list(in: directory) }.value
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
    let unit = burninUnit
    let source = source
    task = Task {
      do {
        let result = try await process.execute(
          directory: directory, executable: executable, burnin: percent, unit: unit,
          heights: heights, source: source)
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
