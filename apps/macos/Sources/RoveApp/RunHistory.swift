import Foundation
import Observation

enum HistoryStatus: String, CaseIterable, Sendable {
  case completed = "Completed"
  case cancelled = "Cancelled"
  case failed = "Failed"
  case interrupted = "Interrupted"
  case incomplete = "Incomplete files"
  case unavailable = "Unavailable"
  case unreadable = "Unreadable record"
}

struct HistoryEntry: Identifiable, Sendable {
  let directory: URL
  let status: HistoryStatus
  let draft: AnalysisDraft?
  let startedAt: Date?
  let elapsedSeconds: Double?
  let completedSteps: UInt64?
  let message: String?
  let files: Set<String>
  let folderAvailable: Bool
  var id: String { directory.path }
  var name: String {
    draft.map { URL(fileURLWithPath: $0.matrixPath).lastPathComponent }
      ?? directory.lastPathComponent
  }

  static func read(_ directory: URL) -> Self {
    let manager = FileManager.default
    let folderAvailable =
      (try? directory.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory == true
    let names = [
      "session.json", "setup.rove.json", "engine.stderr.log", "results/run.json",
      "results/trace.log", "results/trees.nexus",
    ]
    let files = Set(
      names.filter {
        (try? directory.appendingPathComponent($0).resourceValues(forKeys: [.isRegularFileKey]))?
          .isRegularFile == true
      })
    let session = try? SessionRecord.read(directory.appendingPathComponent("session.json"))
    let draft =
      session?.setup
      ?? (try? AnalysisDraft.read(directory.appendingPathComponent("setup.rove.json")))
    var engine = try? EngineRecord.read(directory.appendingPathComponent("results/run.json"))
    if let draft, engine?.matches(draft) != true { engine = nil }
    let status: HistoryStatus
    var message = session?.error
    if !folderAvailable || !manager.isReadableFile(atPath: directory.path) {
      status = .unavailable
      message = "The run folder is missing or unavailable. Restore it, then refresh History."
    } else if let session {
      switch session.status {
      case "completed":
        let outputsPresent = ["results/trace.log", "results/trees.nexus"].allSatisfy {
          let values = try? directory.appendingPathComponent($0).resourceValues(forKeys: [
            .isRegularFileKey, .fileSizeKey,
          ])
          return values?.isRegularFile == true && (values?.fileSize ?? 0) > 0
        }
        if session.exitCode == 0, engine?.status == "completed",
          engine?.completedSteps == UInt64(session.setup.settings.steps), outputsPresent
        {
          status = .completed
        } else {
          status = .incomplete
          message =
            "This run was recorded as completed, but its output files are missing or inconsistent."
        }
      case "cancelled": status = .cancelled
      case "failed": status = .failed
      default:
        status = .interrupted
        message = "This run has no confirmed finish. Its files are partial and cannot be resumed."
      }
    } else {
      status = .unreadable
      message = "The run record is missing, damaged or from an unsupported version."
    }
    let recordedSeconds = session?.elapsedSeconds ?? engine?.wallSeconds
    let seconds = recordedSeconds.flatMap { $0.isFinite && $0 >= 0 ? $0 : nil }
    let engineDate = engine?.startedUnixSeconds.flatMap {
      $0.isFinite && $0 >= 0 ? Date(timeIntervalSince1970: $0) : nil
    }
    return Self(
      directory: directory, status: status, draft: draft,
      startedAt: session?.startedAt ?? engineDate ?? draft?.createdAt,
      elapsedSeconds: seconds, completedSteps: session?.completedSteps ?? engine?.completedSteps,
      message: message, files: files, folderAvailable: folderAvailable)
  }
}

struct HistoryScan: Sendable {
  let entries: [HistoryEntry]
  let unavailableRoots: [String]

  static func read(roots: [URL], known: [URL], defaultRoot: URL? = nil) -> Self {
    let manager = FileManager.default
    var directories = Set(known.map { $0.standardizedFileURL.resolvingSymlinksInPath() })
    var unavailable: [String] = []
    for root in roots {
      do {
        let children = try manager.contentsOfDirectory(
          at: root, includingPropertiesForKeys: [.isDirectoryKey], options: [.skipsHiddenFiles])
        for child in children {
          guard (try? child.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory == true else {
            continue
          }
          if manager.fileExists(atPath: child.appendingPathComponent("session.json").path)
            || manager.fileExists(atPath: child.appendingPathComponent("setup.rove.json").path)
          {
            directories.insert(child.standardizedFileURL.resolvingSymlinksInPath())
          }
        }
      } catch {
        // A new installation has no default run folder until the first analysis.
        if root != defaultRoot || manager.fileExists(atPath: root.path)
          || known.contains(where: { $0.deletingLastPathComponent() == root })
        {
          unavailable.append(root.path)
        }
      }
    }
    let entries = directories.map(HistoryEntry.read).sorted {
      let left = $0.startedAt ?? .distantPast
      let right = $1.startedAt ?? .distantPast
      return left == right ? $0.id < $1.id : left > right
    }
    return Self(entries: entries, unavailableRoots: unavailable)
  }
}

@MainActor @Observable
final class RunHistory {
  private(set) var entries: [HistoryEntry] = []
  private(set) var loading = false
  private(set) var unavailableRoots: [String] = []
  private let defaults: UserDefaults
  private let defaultRoot: URL
  private var generation = UUID()
  private var roots: Set<String>
  private var known: Set<String>

  init(
    defaults: UserDefaults = .standard,
    defaultRoot: URL = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(
      "Rove Runs")
  ) {
    self.defaults = defaults
    self.defaultRoot = defaultRoot.standardizedFileURL.resolvingSymlinksInPath()
    roots = Set(defaults.stringArray(forKey: "historyFolders") ?? [])
    known = Set(defaults.stringArray(forKey: "historyRuns") ?? [])
  }

  func rememberLocation(_ url: URL) {
    guard url.isFileURL else { return }
    roots.insert(url.standardizedFileURL.resolvingSymlinksInPath().path)
    defaults.set(roots.sorted(), forKey: "historyFolders")
  }

  func rememberRun(_ url: URL) {
    guard url.isFileURL else { return }
    known.insert(url.standardizedFileURL.resolvingSymlinksInPath().path)
    defaults.set(known.sorted(), forKey: "historyRuns")
  }

  func reload() async {
    let id = UUID()
    generation = id
    loading = true
    let folders = roots.union([defaultRoot.path]).map { URL(fileURLWithPath: $0) }
    let defaultRoot = defaultRoot
    let saved = known.map { URL(fileURLWithPath: $0) }
    let result = await Task.detached(priority: .utility) {
      HistoryScan.read(roots: folders, known: saved, defaultRoot: defaultRoot)
    }.value
    guard generation == id else { return }
    entries = result.entries
    unavailableRoots = result.unavailableRoots
    known.formUnion(entries.map(\.id))
    defaults.set(known.sorted(), forKey: "historyRuns")
    loading = false
  }
}
