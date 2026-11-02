import AppKit
import SwiftUI
import Testing

@testable import RoveApp

struct FeedbackPreviewTests {
  @Test @MainActor func renderFeedbackChanges() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent()
    let output = root.appendingPathComponent(".local/ui/feedback")
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-feedback-preview-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    let engine = root.appendingPathComponent("target/release/rove")
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    let workspace = Workspace(executable: engine, preferences: preferences)
    workspace.report = report
    workspace.file = matrix
    workspace.outgroup = "T20"
    workspace.settings.apply(SamplingPreset(millions: 30))
    workspace.settings.checkpointEvery = 50_000
    workspace.settings.chains = 2
    let run = AnalysisRun(
      request: RunRequest(
        id: UUID(), executable: engine, directory: output,
        draft: AnalysisDraft(matrix: matrix, report: report, settings: workspace.settings)))
    for chain in 1...2 {
      for index in 0...5 {
        run.receive(
          RunEvent(
            schemaVersion: 1, event: index == 0 ? "started" : "progress",
            step: UInt64(index * 10_000 / chain), totalSteps: 30_000_000,
            elapsedSeconds: Double(index * 5), inputSha256: report.inputSha256,
            posterior: -100, likelihood: -200, prior: 100, message: nil, chain: chain),
          runID: run.id)
      }
    }
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      let strings = preferences.strings
      try await render(
        AnalysisHelpView(topic: .files, strings: strings), name: "files", width: 700, height: 680)
      try await render(
        AnalysisHelpView(topic: .prediction, strings: strings), name: "prediction", width: 700,
        height: 680)
      try await render(
        RunView(workspace: workspace, run: run).padding(28).background(Lab.background)
          .foregroundStyle(Lab.ink), name: "run", width: 820, height: 1300)
      try await render(
        SetupView(workspace: workspace).padding(28).background(Lab.background).foregroundStyle(
          Lab.ink), name: "setup", width: 820, height: 2350)

      @MainActor func render<V: View>(_ content: V, name: String, width: CGFloat, height: CGFloat)
        async throws
      {
        let view = NSHostingView(rootView: content)
        let window = NSWindow(
          contentRect: NSRect(x: 0, y: 0, width: width, height: height),
          styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.appearance = NSAppearance(named: .darkAqua)
        window.contentView = view
        window.orderBack(nil)
        try await Task.sleep(for: .milliseconds(300))
        view.layoutSubtreeIfNeeded()
        let bitmap = try #require(view.bitmapImageRepForCachingDisplay(in: view.bounds))
        view.cacheDisplay(in: view.bounds, to: bitmap)
        try #require(bitmap.representation(using: .png, properties: [:])).write(
          to: output.appendingPathComponent("\(name)-\(language.rawValue).png"))
        window.close()
      }
    }
  }
}
