import AppKit
import SwiftUI
import Testing

@testable import RoveApp

/// Renders the model and review pages with advanced options for visual review; skipped on CI.
struct AdvancedPreviewTests {
  @Test @MainActor func renderAdvancedOptions() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent()
    let output = root.appendingPathComponent(".local/ui/advanced")
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-advanced-preview-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    let engine = root.appendingPathComponent("target/release/rove")
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let workspace = Workspace(executable: engine, preferences: preferences)
    workspace.report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, executable: engine)
    workspace.file = matrix
    workspace.settings.chains = 2
    workspace.settings.stopWhenConverged = true
    workspace.settings.minESS = 300
    workspace.settings.checkpointEvery = 50_000
    workspace.settings.weights = ProposalSchedule.presets[1].weights
    workspace.settings.tuningValues = [MoveOverride(move: "subtree_slide", value: 0.05)]
    for page in [WorkspacePage.model, .review] {
      workspace.page = page
      for language in [AppLanguage.english, .portuguese] {
        preferences.language = language
        let view = NSHostingView(rootView: RootView(workspace: workspace))
        let window = NSWindow(
          contentRect: NSRect(x: 0, y: 0, width: 1240, height: page == .model ? 2300 : 1300),
          styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.appearance = NSAppearance(named: .darkAqua)
        window.contentView = view
        window.orderBack(nil)
        try await Task.sleep(for: .milliseconds(500))
        view.layoutSubtreeIfNeeded()
        let bitmap = try #require(view.bitmapImageRepForCachingDisplay(in: view.bounds))
        view.cacheDisplay(in: view.bounds, to: bitmap)
        try #require(bitmap.representation(using: .png, properties: [:])).write(
          to: output.appendingPathComponent("\(page)-\(language.rawValue).png"))
        window.close()
      }
    }
  }
}
