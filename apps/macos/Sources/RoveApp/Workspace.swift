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
  /// Whether `loading` stands for a tree file being fingerprinted, not a matrix.
  var preparingTrees = false
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
  var outgroup: String? {
    didSet {
      savedURL = nil
      notice = nil
    }
  }
  var constraints: [[String]] {
    GroupConstraints.applied(monophyly, taxa: report?.allTaxa ?? [], outgroup: outgroup)
  }
  var treeSummary: TreeSummaryModel?
  var run: AnalysisRun?
  var showingHistory = false
  var selectedHistoryID: String?
  let history: RunHistory
  let benchmark: Benchmark
  var outputParent = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(
    "Rove Runs")
  var isRunning: Bool { run?.isActive == true || benchmark.isRunning }
  var canOpenBenchmark: Bool {
    accessAllowed() && run?.isActive != true && !loading && treeSummary == nil && !toolsBusy
  }
  func startBenchmark() {
    guard canOpenBenchmark else { return }
    benchmark.start(executable: executable)
  }
  var canStartRun: Bool {
    accessAllowed() && !isRunning && page == .review && report != nil && issue == nil && !loading
      && run == nil
      && !showingHistory
  }
  private var importID = UUID()
  var traceDiagnostics: TraceDiagnosticsModel?
  var treeExplorer: TreeExplorer?
  private let executable: URL
  private let accessAllowed: () -> Bool
  let preferences: AppPreferences
  var strings: AppStrings { preferences.strings }

  init(
    executable: URL = MatrixInspector.executable, preferences: AppPreferences = AppPreferences(),
    history: RunHistory = RunHistory(), benchmark: Benchmark = Benchmark(),
    accessAllowed: @escaping () -> Bool = { true }
  ) {
    self.executable = executable
    self.preferences = preferences
    self.history = history
    self.benchmark = benchmark
    self.accessAllowed = accessAllowed
  }

  var title: String {
    isExample
      ? strings("A small beginning")
      : (file?.deletingPathExtension().lastPathComponent ?? strings("New analysis"))
  }
  var issue: String? {
    guard let report else { return "Import a matrix to continue." }
    if let error = report.samplerError { return error }
    let conditioningError: String? =
      switch settings.conditioning {
      case "variable": report.variableError
      case "informative": report.informativeError
      default: nil
      }
    return conditioningError
      ?? settings.validationError
      ?? settings.dating.validationError(taxa: report.allTaxa)
      ?? GroupConstraints.validate(monophyly, taxa: report.allTaxa, outgroup: outgroup)
  }

  func setDatingEnabled(_ enabled: Bool) {
    if enabled && settings.dating.ages.isEmpty, let taxa = report?.allTaxa {
      settings.dating.ages = Dictionary(uniqueKeysWithValues: taxa.map { ($0, TipAgeInput()) })
    }
    settings.dating.enabled = enabled
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
    let partitioning = preserveSettings ? settings.partitioning : "observed"
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
        if try await Task.detached { try Self.holdsTrees(url) }.value {
          throw ImportError.message(
            "This file holds trees, not a character matrix. Summarize and explore it with File → Summarize Tree File."
          )
        }
        guard (values.fileSize ?? 0) <= 32 * 1024 * 1024 else {
          throw ImportError.message("The matrix preview supports files up to 32 MB.")
        }
        let imported = try await MatrixInspector.inspect(
          url, gapsAsMissing: gaps, partitioning: partitioning, executable: executable)
        guard importID == id else { return }
        if !preserveSettings {
          settings = AnalysisSettings()
          monophyly = []
          outgroup = nil
        }
        run = nil
        showingHistory = false
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

  /// Whether a NEXUS file opens a TREES block before any characters: a tree
  /// sample rather than a matrix. Only the first 256 KB are read, with
  /// bracketed comments removed.
  nonisolated static func holdsTrees(_ url: URL) throws -> Bool {
    let handle = try FileHandle(forReadingFrom: url)
    defer { try? handle.close() }
    let head = String(decoding: try handle.read(upToCount: 256 * 1024) ?? Data(), as: UTF8.self)
      .lowercased().replacing(/\[[^\]]*\]/, with: " ")
    let trees = head.firstRange(of: /begin\s+trees\s*;/)?.lowerBound
    let characters = head.firstRange(of: /begin\s+(characters|data)\s*;/)?.lowerBound
    guard let trees else { return false }
    return characters.map { trees < $0 } ?? true
  }

  func summarizeTreeFile() {
    guard !isRunning, !loading, treeSummary == nil else { return }
    let panel = NSOpenPanel()
    panel.title = strings("Choose a tree file")
    panel.message = strings(
      "Open a NEXUS tree sample written by Rove, BEAST 2 or another program. Your data stays on this Mac."
    )
    panel.prompt = strings("Summarize")
    panel.canChooseDirectories = false
    panel.allowsMultipleSelection = false
    if panel.runModal() == .OK, let url = panel.url { openTreeFile(url) }
  }

  /// Prepares the folder that keeps the file's summaries, lists it in History
  /// and opens the summary sheet. The file is fingerprinted every time, so a
  /// changed file gets its own folder.
  func openTreeFile(_ url: URL) {
    guard !isRunning, !loading, treeSummary == nil else { return }
    let id = UUID()
    importID = id
    loading = true
    preparingTrees = true
    error = nil
    errorDetails = nil
    let parent = outputParent
    Task {
      do {
        let container = try await Task.detached { try TreeSource.prepare(url, under: parent) }
          .value
        guard importID == id else { return }
        history.rememberLocation(parent)
        history.rememberRun(container)
        treeSummary = TreeSummaryModel(
          directory: container, executable: executable, source: .file(url))
        await history.reload()
      } catch {
        guard importID == id else { return }
        if case ImportError.message(let key) = error {
          self.error = key
        } else {
          self.error = "The tree file could not be opened."
          errorDetails = error.localizedDescription
        }
      }
      if importID == id {
        loading = false
        preparingTrees = false
      }
    }
  }

  func reset() {
    guard !isRunning else { return }
    run = nil
    showingHistory = false
    monophyly = []
    outgroup = nil
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
        matrix: file, report: report, settings: settings, monophyly: monophyly, outgroup: outgroup)
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
    do {
      let command = try settings.command(
        executable: executable, matrix: file, output: output, monophyly: constraints)
      NSPasteboard.general.clearContents()
      NSPasteboard.general.setString(command, forType: .string)
      notice = "Command copied. Paste it in Terminal to run this analysis."
    } catch {
      self.error = (error as? ImportError)?.errorDescription ?? "The setup could not be saved."
    }
  }

  /// Routes a dropped or double-clicked file: a setup, a tree sample or a matrix.
  func open(_ url: URL) {
    if url.lastPathComponent.hasSuffix(".rove.json") {
      loadSetup(url)
    } else if url.pathExtension.lowercased() == "trees" {
      openTreeFile(url)
    } else {
      load(url)
    }
  }

  func chooseSetup() {
    guard !isRunning else { return }
    let panel = NSOpenPanel()
    panel.title = strings("Open saved setup")
    panel.allowedContentTypes = [.json]
    panel.allowsMultipleSelection = false
    if panel.runModal() == .OK, let url = panel.url { loadSetup(url) }
  }

  func loadSetup(_ url: URL) { restoreSetup(url: url, stored: nil) }

  func reuseSetup(_ entry: HistoryEntry) {
    guard let draft = entry.draft, !isRunning, !loading else { return }
    restoreSetup(url: nil, stored: draft)
  }

  private func restoreSetup(url: URL?, stored: AnalysisDraft?) {
    guard !isRunning else { return }
    let id = UUID()
    importID = id
    loading = true
    error = nil
    errorDetails = nil
    notice = nil
    Task {
      do {
        let draft = try await Task.detached {
          if let stored {
            try stored.validate()
            return stored
          }
          guard let url else { throw ImportError.message("The setup could not be opened.") }
          return try AnalysisDraft.read(url)
        }.value
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
          matrix, gapsAsMissing: draft.settings.gapsAsMissing,
          partitioning: draft.settings.partitioning, executable: executable)
        guard imported.inputSha256 == draft.matrixSHA256 else { throw RunFailure.changedInput }
        try draft.validateTaxa(imported.allTaxa)
        guard importID == id else { return }
        run = nil
        settings = draft.settings
        monophyly = draft.monophyly
        outgroup = draft.outgroup
        report = imported
        file = matrix
        isExample = false
        page = .review
        showingHistory = false
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
    if panel.runModal() == .OK, let url = panel.url {
      outputParent = url
      history.rememberLocation(url)
    }
  }

  func startRun() {
    guard canStartRun, let file, let report else { return }
    let id = UUID()
    let request = RunRequest(
      id: id, executable: executable,
      directory: outputParent.appendingPathComponent("analysis-\(id.uuidString)"),
      draft: AnalysisDraft(
        matrix: file, report: report, settings: settings, monophyly: monophyly, outgroup: outgroup))
    let analysis = AnalysisRun(request: request)
    run = analysis
    error = nil
    notice = nil
    history.rememberLocation(outputParent)
    history.rememberRun(request.directory)
    analysis.start { [history] in await history.reload() }
  }

  func resumeRun(_ entry: HistoryEntry) {
    guard accessAllowed(), !isRunning, !loading, let draft = entry.draft, entry.checkpoint != nil
    else { return }
    let request = RunRequest(
      id: UUID(), executable: executable, directory: entry.directory, draft: draft, resume: true)
    let analysis = AnalysisRun(request: request)
    run = analysis
    error = nil
    notice = nil
    showingHistory = false
    analysis.start { [history] in await history.reload() }
  }

  func dismissRun() {
    guard !isRunning else { return }
    run = nil
    showingHistory = false
    page = .review
  }

  func cancelAndWait() async {
    if benchmark.isRunning {
      benchmark.cancel()
      await benchmark.waitForExit()
    }
    guard let run, run.isActive else { return }
    run.cancel()
    await run.waitForExit()
  }

  func applyGroup(_ selection: GroupSelection) -> Bool {
    guard !isRunning, let report else { return false }
    guard selection.selected.isSubset(of: Set(report.allTaxa)),
      selection.index.map({ monophyly.indices.contains($0) }) ?? true
    else { return false }
    let groups = selection.groups(replacing: monophyly, taxa: report.allTaxa)
    guard GroupConstraints.validate(groups, taxa: report.allTaxa, outgroup: outgroup) == nil else {
      return false
    }
    monophyly = groups
    return true
  }

  func openDiagnostics(in directory: URL) {
    traceDiagnostics = TraceDiagnosticsModel(directory: directory, executable: executable)
  }

  func openTree(_ directory: URL, run: URL) {
    treeExplorer = TreeExplorer(directory: directory, runDirectory: run, executable: executable)
  }

  func summarizeTrees(in directory: URL) {
    treeSummary = TreeSummaryModel(directory: directory, executable: executable)
  }

  func runComparison(for entry: HistoryEntry) -> RunComparisonModel {
    RunComparisonModel(entry: entry, executable: executable)
  }

  /// Checks and estimates keep running while History shows another run.
  var predictiveChecks: [String: PredictiveCheckModel] = [:]
  var marginalEstimates: [String: MarginalLikelihoodModel] = [:]
  var toolsBusy: Bool {
    predictiveChecks.values.contains(where: \.busy)
      || marginalEstimates.values.contains(where: \.busy)
  }

  func predictiveCheck(for entry: HistoryEntry) -> PredictiveCheckModel {
    if let model = predictiveChecks[entry.id] { return model }
    let model = PredictiveCheckModel(entry: entry, executable: executable)
    predictiveChecks[entry.id] = model
    return model
  }

  func marginalLikelihood(for entry: HistoryEntry) -> MarginalLikelihoodModel {
    if let model = marginalEstimates[entry.id] { return model }
    let model = MarginalLikelihoodModel(entry: entry, executable: executable)
    marginalEstimates[entry.id] = model
    return model
  }

  func openHistory() {
    showingHistory = true
    selectedHistoryID = nil
    Task { await history.reload() }
  }

  func returnToWorkspace() { showingHistory = false }

  func addHistoryFolder() {
    let panel = NSOpenPanel()
    panel.title = strings("Add a run folder")
    panel.message = strings("Choose the output folder of an earlier Rove analysis.")
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.allowsMultipleSelection = false
    guard panel.runModal() == .OK, let url = panel.url else { return }
    guard
      ["session.json", "setup.rove.json"].contains(where: {
        FileManager.default.fileExists(atPath: url.appendingPathComponent($0).path)
      })
    else {
      error = "This folder does not contain a saved Rove run."
      errorDetails = nil
      return
    }
    history.rememberRun(url)
    Task { await history.reload() }
  }

}
