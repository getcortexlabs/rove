import AppKit
import Observation
import SwiftUI
import UniformTypeIdentifiers

enum WorkspacePage: String, CaseIterable {
  case matrix = "Matrix"
  case model = "Model"
  case review = "Review"
  var number: Int { Self.allCases.firstIndex(of: self)! + 1 }
}

@MainActor @Observable
final class Workspace {
  var report: MatrixReport?
  var file: URL?
  var page = WorkspacePage.matrix
  var settings = AnalysisSettings() {
    didSet {
      savedURL = nil
      notice = nil
    }
  }
  var loading = false
  var error: String?
  var errorDetails: String?
  var notice: String?
  var savedURL: URL?
  var isExample = false
  var monophyly: [[String]] = [] {
    didSet {
      savedURL = nil
      notice = nil
    }
  }
  var run: AnalysisRun?
  var outputParent = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(
    "Rove Runs")
  var isRunning: Bool { run?.isActive == true }
  var canStartRun: Bool {
    page == .review && report != nil && issue == nil && !loading && run == nil
  }
  private var importID = UUID()
  private let executable: URL
  let preferences: AppPreferences
  var strings: AppStrings { preferences.strings }

  init(executable: URL = MatrixInspector.executable, preferences: AppPreferences = AppPreferences())
  {
    self.executable = executable
    self.preferences = preferences
  }

  var title: String {
    isExample
      ? strings("A small beginning")
      : (file?.deletingPathExtension().lastPathComponent ?? strings("New analysis"))
  }
  var issue: String? {
    guard let report else { return "Import a matrix to continue." }
    return report.samplerError ?? (settings.conditioning == "variable" ? report.variableError : nil)
      ?? settings.validationError ?? GroupConstraints.validate(monophyly, taxa: report.allTaxa)
  }

  func canOpen(_ destination: WorkspacePage) -> Bool {
    guard !isRunning && !loading else { return false }
    return switch destination {
    case .matrix: true
    case .model: report != nil && report?.samplerError == nil
    case .review: report != nil && issue == nil
    }
  }

  func chooseFile() {
    guard !isRunning else { return }
    let panel = NSOpenPanel()
    panel.title = strings("Choose a morphological matrix")
    panel.message = strings("Open a NEXUS file. Your data stays on this Mac.")
    panel.prompt = strings("Open matrix")
    panel.canChooseDirectories = false
    panel.allowsMultipleSelection = false
    if panel.runModal() == .OK, let url = panel.url { load(url) }
  }

  func loadExample() {
    guard let url = Lab.resource("Example", extension: "nex") else { return }
    load(url, example: true)
  }

  func load(_ url: URL, example: Bool = false, preserveSettings: Bool = false) {
    guard !isRunning else { return }
    let id = UUID()
    importID = id
    loading = true
    error = nil
    errorDetails = nil
    notice = nil
    let gaps = preserveSettings ? settings.gapsAsMissing : false
    Task {
      let scoped = url.startAccessingSecurityScopedResource()
      defer { if scoped { url.stopAccessingSecurityScopedResource() } }
      do {
        guard url.isFileURL else {
          throw ImportError.message("Choose a local NEXUS file from your Mac.")
        }
        let values = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
        guard values.isRegularFile == true else {
          throw ImportError.message("Choose a NEXUS file, rather than a folder.")
        }
        guard (values.fileSize ?? 0) <= 32 * 1024 * 1024 else {
          throw ImportError.message("The matrix preview supports files up to 32 MB.")
        }
        let imported = try await MatrixInspector.inspect(
          url, gapsAsMissing: gaps, executable: executable)
        guard importID == id else { return }
        if !preserveSettings {
          settings = AnalysisSettings()
          monophyly = []
        }
        run = nil
        report = imported
        file = url
        isExample = example
        page = .matrix
        savedURL = nil
      } catch {
        guard importID == id else { return }
        if let importError = error as? ImportError {
          switch importError {
          case .message(let key): self.error = key
          case .readerFailed(let details):
            self.error = "The matrix could not be read."
            errorDetails = details
          }
        } else {
          self.error = "The file could not be opened."
          errorDetails = error.localizedDescription
        }
        // A failed gap-policy change must not leave an old report paired with new settings.
        if preserveSettings { report = nil }
      }
      if importID == id { loading = false }
    }
  }

  func reloadGapPolicy() {
    if let file { load(file, example: isExample, preserveSettings: true) }
  }

  func reset() {
    guard !isRunning else { return }
    run = nil
    monophyly = []
    importID = UUID()
    report = nil
    file = nil
    page = .matrix
    settings = AnalysisSettings()
    loading = false
    error = nil
    errorDetails = nil
    notice = nil
    savedURL = nil
    isExample = false
  }

  func saveDraft() {
    guard let file, let report, issue == nil else { return }
    let panel = NSSavePanel()
    panel.title = strings("Save analysis setup")
    panel.prompt = strings("Save setup")
    panel.nameFieldStringValue = "\(title).rove.json"
    panel.allowedContentTypes = [.json]
    panel.message =
      strings(
        "Save the matrix location, its fingerprint and your settings. This does not run an analysis."
      )
    guard panel.runModal() == .OK, let url = panel.url else { return }
    do {
      let draft = AnalysisDraft(
        matrix: file, report: report, settings: settings, monophyly: monophyly)
      try draft.write(to: url)
      savedURL = url
      notice = "Setup saved. Your next discovery has a starting point."
    } catch {
      self.error = "The setup could not be saved."
      errorDetails = error.localizedDescription
    }
  }

  func copyCommand() {
    guard let file, issue == nil else { return }
    let output = FileManager.default.homeDirectoryForCurrentUser
      .appendingPathComponent("Rove Runs")
      .appendingPathComponent("analysis-\(UUID().uuidString.prefix(8))")
    let command = settings.command(
      executable: executable, matrix: file, output: output, monophyly: monophyly)
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString(command, forType: .string)
    notice = "Command copied. Paste it in Terminal to run this analysis."
  }

  func open(_ url: URL) {
    if url.lastPathComponent.hasSuffix(".rove.json") { loadSetup(url) } else { load(url) }
  }

  func chooseSetup() {
    guard !isRunning else { return }
    let panel = NSOpenPanel()
    panel.title = strings("Open saved setup")
    panel.allowedContentTypes = [.json]
    panel.allowsMultipleSelection = false
    if panel.runModal() == .OK, let url = panel.url { loadSetup(url) }
  }

  func loadSetup(_ url: URL) {
    guard !isRunning else { return }
    let id = UUID()
    importID = id
    loading = true
    error = nil
    errorDetails = nil
    notice = nil
    Task {
      do {
        let draft = try await Task.detached { try AnalysisDraft.read(url) }.value
        let matrix = URL(fileURLWithPath: draft.matrixPath)
        guard FileManager.default.isReadableFile(atPath: matrix.path) else {
          throw ImportError.message(
            "The saved matrix is missing or unreadable. Restore it at its original location, or import another matrix."
          )
        }
        let values = try matrix.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
        guard values.isRegularFile == true, (values.fileSize ?? 0) <= 32 * 1024 * 1024 else {
          throw ImportError.message("The matrix preview supports files up to 32 MB.")
        }
        let imported = try await MatrixInspector.inspect(
          matrix, gapsAsMissing: draft.settings.gapsAsMissing, executable: executable)
        guard imported.inputSha256 == draft.matrixSHA256 else { throw RunFailure.changedInput }
        if let issue = GroupConstraints.validate(draft.monophyly, taxa: imported.allTaxa) {
          throw ImportError.message(issue)
        }
        guard importID == id else { return }
        run = nil
        settings = draft.settings
        monophyly = draft.monophyly
        report = imported
        file = matrix
        isExample = false
        page = .review
        savedURL = url
        notice = "Setup reopened. Review it before starting a new analysis."
      } catch {
        guard importID == id else { return }
        self.error =
          (error as? RunFailure)?.errorDescription
          ?? (error as? ImportError)?.errorDescription ?? "The setup could not be opened."
        if !(error is RunFailure) && !(error is ImportError) {
          errorDetails = error.localizedDescription
        }
      }
      if importID == id { loading = false }
    }
  }

  func chooseOutputFolder() {
    let panel = NSOpenPanel()
    panel.title = strings("Choose where to save runs")
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.canCreateDirectories = true
    panel.allowsMultipleSelection = false
    panel.directoryURL = outputParent
    if panel.runModal() == .OK, let url = panel.url { outputParent = url }
  }

  func startRun() {
    guard canStartRun, let file, let report else { return }
    let id = UUID()
    let request = RunRequest(
      id: id, executable: executable,
      directory: outputParent.appendingPathComponent("analysis-\(id.uuidString)"),
      draft: AnalysisDraft(matrix: file, report: report, settings: settings, monophyly: monophyly))
    let analysis = AnalysisRun(request: request)
    run = analysis
    error = nil
    notice = nil
    analysis.start()
  }

  func dismissRun() {
    guard !isRunning else { return }
    run = nil
    page = .review
  }

  func cancelAndWait() async {
    guard let run, run.isActive else { return }
    run.cancel()
    await run.waitForExit()
  }

}
