import AppKit
import SwiftUI
import Testing

@testable import RoveApp

/// Renders History with a saved predictive check and marginal-likelihood
/// estimates for visual review; skipped on CI.
struct ModelToolsPreviewTests {
  @Test @MainActor func renderModelTools() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent()
    let output = root.appendingPathComponent(".local/ui/model-tools")
    try? FileManager.default.removeItem(at: output)
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-model-tools-preview-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    let history = RunHistory(defaults: defaults, defaultRoot: output)
    let engine = root.appendingPathComponent("target/release/rove")
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var directories: [URL] = []
    for (name, seed, clock) in [("relaxed-42", "42", "reference"), ("strict-43", "43", "strict")] {
      var settings = AnalysisSettings()
      settings.steps = 20_000
      settings.sampleEvery = 100
      settings.seed = seed
      settings.clock = clock
      let directory = output.appendingPathComponent(name)
      let request = RunRequest(
        id: UUID(), executable: engine, directory: directory,
        draft: AnalysisDraft(
          matrix: matrix, report: report, settings: settings,
          monophyly: [Array(report.allTaxa.prefix(19))], outgroup: "T20"))
      #expect(await RunProcess().execute(request) { _ in }.status == .completed)
      directories.append(directory)
    }
    await history.reload()
    let workspace = Workspace(executable: engine, preferences: preferences, history: history)
    let relaxed = try #require(history.entries.first { $0.directory.path == directories[0].path })
    let strict = try #require(history.entries.first { $0.directory.path == directories[1].path })
    let check = workspace.predictiveCheck(for: relaxed)
    check.replicates = "20"
    check.start()
    await check.waitForExit()
    #expect(check.error == nil, Comment(rawValue: check.details ?? ""))
    for entry in [relaxed, strict] {
      let estimate = workspace.marginalLikelihood(for: entry)
      estimate.powers = "6"
      estimate.steps = "2000"
      estimate.burnin = "25"
      estimate.start()
      await estimate.waitForExit()
      #expect(estimate.error == nil, Comment(rawValue: estimate.details ?? ""))
    }
    workspace.showingHistory = true
    workspace.selectedHistoryID = relaxed.id
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      let view = NSHostingView(rootView: RootView(workspace: workspace))
      let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: 1240, height: 3200),
        styleMask: [.borderless], backing: .buffered, defer: false)
      window.isReleasedWhenClosed = false
      window.appearance = NSAppearance(named: .darkAqua)
      window.contentView = view
      window.orderBack(nil)
      try await Task.sleep(for: .milliseconds(800))
      view.layoutSubtreeIfNeeded()
      let bitmap = try #require(view.bitmapImageRepForCachingDisplay(in: view.bounds))
      view.cacheDisplay(in: view.bounds, to: bitmap)
      try #require(bitmap.representation(using: .png, properties: [:])).write(
        to: output.appendingPathComponent("history-\(language.rawValue).png"))
      window.close()
    }
  }
}
