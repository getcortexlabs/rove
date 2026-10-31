import Foundation
import Testing

@testable import RoveApp

struct TreeFileTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }

  /// Two trees in BEAST 2's shape: a rate on every branch and a zero-length root stem.
  static let beast = """
    #NEXUS
    Begin trees;
    Translate 1 A, 2 B, 3 C;
    tree STATE_0 = ((1[&rate=0.9]:1,2[&rate=1.1]:1)[&rate=1.0]:1,3[&rate=0.8]:2)[&rate=1.0]:0.0;
    tree STATE_100 = ((1[&rate=1.2]:1,3[&rate=0.7]:1)[&rate=0.9]:1,2[&rate=1.0]:2)[&rate=1.0]:0.0;
    End;

    """

  @Test @MainActor func treeFilesAreSummarizedListedAndExplored() async throws {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-tree-file-\(UUID())")
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: folder) }
    let runs = folder.appendingPathComponent("runs")
    let file = folder.appendingPathComponent("four.trees")
    try Data(contentsOf: root.appendingPathComponent("fixtures/summary/four-taxa.trees"))
      .write(to: file)

    let container = try TreeSource.prepare(file, under: runs)
    #expect(container.deletingLastPathComponent().path == runs.path)
    #expect(container.lastPathComponent.hasPrefix("trees-four-"))
    let source = try TreeSource.read(container.appendingPathComponent(TreeSource.fileName))
    #expect(source.name == "four.trees" && source.path == file.path)
    #expect(source.sha256 == (try TreeSource.fingerprint(file)))
    // The same content keeps the same folder and record; a new path updates the record.
    #expect(try TreeSource.prepare(file, under: runs).path == container.path)
    #expect(try TreeSource.read(container.appendingPathComponent(TreeSource.fileName)) == source)
    let copy = folder.appendingPathComponent("copy.trees")
    try FileManager.default.copyItem(at: file, to: copy)
    let copyContainer = try TreeSource.prepare(copy, under: runs)
    #expect(copyContainer.lastPathComponent.hasPrefix("trees-copy-"))
    try FileManager.default.removeItem(at: copyContainer)
    let record = container.appendingPathComponent(TreeSource.fileName)
    try "{}".write(to: record, atomically: true, encoding: .utf8)
    #expect(throws: (any Error).self) { try TreeSource.read(record) }
    #expect(try TreeSource.prepare(file, under: runs).path == container.path)
    #expect(try TreeSource.read(record).path == file.path)

    let model = TreeSummaryModel(directory: container, executable: engine, source: .file(file))
    model.burnin = "25"
    model.heights = "keep"
    model.start()
    await model.waitForExit()
    #expect(model.error == nil, Comment(rawValue: model.details ?? ""))
    let saved = try #require(model.summaries.first)
    #expect(saved.record.totalSamples == 4 && saved.record.discardedSamples == 1)
    #expect(saved.directory.deletingLastPathComponent().lastPathComponent == "summaries")
    #expect(SavedSummary.list(in: container).map(\.id) == [saved.id])

    let entry = HistoryEntry.read(container)
    #expect(entry.status == .treeFile && entry.name == "four.trees" && entry.draft == nil)
    #expect(entry.source == source && entry.startedAt == source.createdAt)
    let scan = HistoryScan.read(roots: [runs], known: [], defaultRoot: runs)
    #expect(scan.entries.map(\.id) == [entry.id])

    let explorer = TreeExplorer(
      directory: saved.directory, runDirectory: container, executable: engine)
    await explorer.load()
    #expect(explorer.error == nil, Comment(rawValue: explorer.details ?? ""))
    #expect(explorer.outgroup == nil && explorer.document?.nodes.count == 7)
    #expect(explorer.runName == "four.trees")

    // A run folder keeps its own record even beside a tree-source record.
    let run = folder.appendingPathComponent("run")
    let report = try await MatrixInspector.inspect(
      root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"), gapsAsMissing: false,
      executable: engine)
    var settings = AnalysisSettings()
    settings.steps = 500
    settings.sampleEvery = 100
    let request = RunRequest(
      id: UUID(), executable: engine, directory: run,
      draft: AnalysisDraft(
        matrix: root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"),
        report: report, settings: settings))
    #expect(await RunProcess().execute(request) { _ in }.status == .completed)
    try FileManager.default.copyItem(
      at: container.appendingPathComponent(TreeSource.fileName),
      to: run.appendingPathComponent(TreeSource.fileName))
    let both = HistoryEntry.read(run)
    #expect(both.status == .completed && both.source == nil && both.draft != nil)

    // A file in BEAST's shape summarizes through the same path.
    let beast = folder.appendingPathComponent("beast.trees")
    try Self.beast.write(to: beast, atomically: true, encoding: .utf8)
    let beastContainer = try TreeSource.prepare(beast, under: runs)
    #expect(beastContainer.path != container.path)
    let beastModel = TreeSummaryModel(
      directory: beastContainer, executable: engine, source: .file(beast))
    beastModel.burnin = "0"
    beastModel.start()
    await beastModel.waitForExit()
    #expect(beastModel.error == nil, Comment(rawValue: beastModel.details ?? ""))
    #expect(beastModel.summaries.first?.record.totalSamples == 2)

    // A moved file is reported; the saved summaries and the History entry stay.
    try FileManager.default.removeItem(at: file)
    let missing = TreeSummaryModel(directory: container, executable: engine, source: .file(file))
    missing.start()
    await missing.waitForExit()
    #expect(missing.error != nil && missing.details?.contains("recorded path") == true)
    #expect(HistoryEntry.read(container).status == .treeFile)
    #expect(SavedSummary.list(in: container).count == 1)
  }

  @Test @MainActor func matrixImportRecognizesTreeFiles() async throws {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-tree-import-\(UUID())")
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: folder) }
    let trees = folder.appendingPathComponent("sample.trees")
    try Self.beast.write(to: trees, atomically: true, encoding: .utf8)
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    #expect(try Workspace.holdsTrees(trees))
    #expect(try !Workspace.holdsTrees(matrix))
    let spaced = folder.appendingPathComponent("spaced.nex")
    try "#NEXUS\n[begin characters;]\nBEGIN\tTREES ;\nEnd;\n"
      .write(to: spaced, atomically: true, encoding: .utf8)
    #expect(try Workspace.holdsTrees(spaced))
    let commented = folder.appendingPathComponent("commented.nex")
    try "#NEXUS\n[begin trees;]\nBegin data;\nEnd;\n"
      .write(to: commented, atomically: true, encoding: .utf8)
    #expect(try !Workspace.holdsTrees(commented))
    // Rove's own matrix files may carry a TREES block after the characters.
    let both = folder.appendingPathComponent("both.nex")
    try (String(contentsOf: matrix, encoding: .utf8) + "\nBegin trees; End;\n")
      .write(to: both, atomically: true, encoding: .utf8)
    #expect(try !Workspace.holdsTrees(both))

    let suite = "rove-tree-import-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    let history = RunHistory(defaults: defaults, defaultRoot: folder.appendingPathComponent("runs"))
    let workspace = Workspace(executable: engine, preferences: preferences, history: history)
    workspace.outputParent = folder.appendingPathComponent("runs")
    // A tree sample handed to the matrix import opens the summary sheet instead.
    workspace.load(trees)
    for _ in 0..<200 where workspace.treeSummary == nil {
      try await Task.sleep(for: .milliseconds(25))
    }
    #expect(workspace.error == nil && workspace.report == nil)
    let sheet = try #require(workspace.treeSummary)
    #expect(sheet.source == .file(trees))
    #expect(sheet.directory.lastPathComponent.hasPrefix("trees-sample-"))
    #expect(!workspace.loading && !workspace.preparingTrees)
    await history.reload()
    #expect(history.entries.contains { $0.status == .treeFile && $0.name == "sample.trees" })
  }
}
