import AppKit
import SwiftUI
import Testing

@testable import RoveApp

/// Renders the tree explorer on local summaries for visual review; skipped on CI.
struct ExplorerPreviewTests {
  @Test @MainActor func renderTreeExplorer() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent()
    let output = root.appendingPathComponent(".local/ui/explorer")
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-explorer-preview-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    let engine = root.appendingPathComponent("target/release/rove")
    let samples = root.appendingPathComponent(".local/issue35")
    // The summaries come from local runs that are not part of the repository.
    guard FileManager.default.fileExists(atPath: samples.appendingPathComponent("summary-20").path),
      FileManager.default.fileExists(atPath: engine.path)
    else { return }
    let cases: [(String, String, String)] = [
      ("reference", "summary-20", ".local/issue42/final/reference-mk-42201-0.1/chain-01"),
      ("dated", "summary-dated", ".local/issue42/final/dated-estimated-42121-0.1/chain-01"),
    ]
    for (name, summary, run) in cases {
      let model = TreeExplorer(
        directory: samples.appendingPathComponent(summary),
        runDirectory: root.appendingPathComponent(run), executable: engine)
      await model.load()
      try #require(model.document != nil, Comment(rawValue: model.details ?? ""))
      model.selected = model.layout?.order.first { model.document?.nodes[$0].children != nil }
      for (size, suffix) in [(CGSize(width: 1240, height: 820), ""), (CGSize(width: 900, height: 660), "-compact")] {
        let view = NSHostingView(
          rootView: TreeExplorerView(model: model, preferences: preferences))
        let window = NSWindow(
          contentRect: NSRect(origin: .zero, size: size), styleMask: [.borderless],
          backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.appearance = NSAppearance(named: .darkAqua)
        window.contentView = view
        window.orderBack(nil)
        for language in [AppLanguage.english, .portuguese] {
          preferences.language = language
          try await Task.sleep(for: .milliseconds(450))
          view.layoutSubtreeIfNeeded()
          let bitmap = try #require(view.bitmapImageRepForCachingDisplay(in: view.bounds))
          view.cacheDisplay(in: view.bounds, to: bitmap)
          try #require(bitmap.representation(using: .png, properties: [:])).write(
            to: output.appendingPathComponent("\(name)\(suffix)-\(language.rawValue).png"))
        }
        window.close()
      }
      if let tree = model.document, let layout = model.layout {
        let figure = TreeFigure(
          tree: tree, layout: layout, viewport: CGSize(width: 1240, height: 820),
          collapsed: layout.collapsed, selected: model.selected, showSupport: true,
          strings: preferences.strings)
        try figure.svg(runName: name).write(
          to: output.appendingPathComponent("\(name).svg"), atomically: true, encoding: .utf8)
      }
    }
  }
}
