import AppKit
import SwiftUI
import Testing

@testable import RoveApp

/// Renders History with a summarized tree file for visual review; skipped on CI.
struct TreeFilePreviewTests {
  @Test @MainActor func renderTreeFileHistory() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent()
    let output = root.appendingPathComponent(".local/ui/tree-file")
    try? FileManager.default.removeItem(at: output)
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-tree-file-preview-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    let history = RunHistory(defaults: defaults, defaultRoot: output)
    let engine = root.appendingPathComponent("target/release/rove")
    guard FileManager.default.fileExists(atPath: engine.path) else { return }
    let file = output.appendingPathComponent("Britto_2003.trees")
    try Data(contentsOf: root.appendingPathComponent("fixtures/summary/four-taxa.trees"))
      .write(to: file)
    let container = try TreeSource.prepare(file, under: output)
    let model = TreeSummaryModel(directory: container, executable: engine, source: .file(file))
    model.burnin = "25"
    model.start()
    await model.waitForExit()
    #expect(model.error == nil, Comment(rawValue: model.details ?? ""))
    await history.reload()
    let workspace = Workspace(executable: engine, preferences: preferences, history: history)
    workspace.showingHistory = true
    workspace.selectedHistoryID = try #require(history.entries.first { $0.status == .treeFile }).id
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      try await render(
        RootView(workspace: workspace), size: NSSize(width: 1240, height: 1100),
        to: output.appendingPathComponent("history-\(language.rawValue).png"))
      try await render(
        TreeSummaryView(model: model, strings: preferences.strings) { _ in },
        size: NSSize(width: 620, height: 720),
        to: output.appendingPathComponent("sheet-\(language.rawValue).png"))
    }
    workspace.showingHistory = false
    try await render(
      RootView(workspace: workspace), size: NSSize(width: 1240, height: 900),
      to: output.appendingPathComponent("welcome-pt-BR.png"))
  }

  @MainActor private func render(_ view: some View, size: NSSize, to file: URL) async throws {
    let host = NSHostingView(rootView: view)
    let window = NSWindow(
      contentRect: NSRect(origin: .zero, size: size), styleMask: [.borderless],
      backing: .buffered, defer: false)
    window.isReleasedWhenClosed = false
    window.appearance = NSAppearance(named: .darkAqua)
    window.contentView = host
    window.orderBack(nil)
    try await Task.sleep(for: .milliseconds(600))
    host.layoutSubtreeIfNeeded()
    let bitmap = try #require(host.bitmapImageRepForCachingDisplay(in: host.bounds))
    host.cacheDisplay(in: host.bounds, to: bitmap)
    try #require(bitmap.representation(using: .png, properties: [:])).write(to: file)
    window.close()
  }
}
