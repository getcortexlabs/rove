import AppKit
import SwiftUI
import Testing

@testable import RoveApp

struct DatingPreviewTests {
  @Test @MainActor func renderDatingControls() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let directory = root.appendingPathComponent(".local/ui/dating")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-dating-preview-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults)
    let engine = root.appendingPathComponent("target/release/rove")
    let matrix = root.appendingPathComponent("fixtures/validation/dating/matrix.nex")
    let workspace = Workspace(executable: engine, preferences: preferences)
    workspace.file = matrix
    workspace.report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, executable: engine)
    workspace.settings.dating = DatingTests.example
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      workspace.page = .model
      try await capture(
        RootView(workspace: workspace), size: CGSize(width: 1080, height: 700),
        to: directory.appendingPathComponent("model-\(language.rawValue).png"))
      try await capture(
        DatingSection(workspace: workspace).padding(24).background(Lab.background),
        size: CGSize(width: 800, height: 820),
        to: directory.appendingPathComponent("controls-\(language.rawValue).png"))
      try await capture(
        TipAgeEditor(workspace: workspace), size: CGSize(width: 730, height: 620),
        to: directory.appendingPathComponent("ages-\(language.rawValue).png"))
      workspace.page = .review
      try await capture(
        RootView(workspace: workspace), size: CGSize(width: 1080, height: 700),
        to: directory.appendingPathComponent("review-\(language.rawValue).png"))
      try await capture(
        LabPanel { DatingSummary(dating: workspace.settings.dating, strings: workspace.strings) }
          .padding(24).background(Lab.background), size: CGSize(width: 700, height: 500),
        to: directory.appendingPathComponent("summary-\(language.rawValue).png"))
    }
  }
  @MainActor private func capture<V: View>(_ content: V, size: CGSize, to url: URL) async throws {
    let view = NSHostingView(
      rootView: content.preferredColorScheme(.dark).foregroundStyle(Lab.ink).tint(Lab.accent)
        .frame(width: size.width, height: size.height, alignment: .top))
    let window = NSWindow(
      contentRect: NSRect(origin: .zero, size: size),
      styleMask: [.borderless], backing: .buffered, defer: false)
    window.isReleasedWhenClosed = false
    window.appearance = NSAppearance(named: .darkAqua)
    window.contentView = view
    window.orderBack(nil)
    defer { window.close() }
    try await Task.sleep(for: .milliseconds(350))
    view.layoutSubtreeIfNeeded()
    let bitmap = try #require(view.bitmapImageRepForCachingDisplay(in: view.bounds))
    view.cacheDisplay(in: view.bounds, to: bitmap)
    try #require(bitmap.representation(using: .png, properties: [:])).write(to: url)
  }
}
