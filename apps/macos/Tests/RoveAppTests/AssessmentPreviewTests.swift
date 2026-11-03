import AppKit
import SwiftUI
import Testing

@testable import RoveApp

struct AssessmentPreviewTests {
  @Test @MainActor func renderAssessmentAndPairedDistribution() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent()
    let output = root.appendingPathComponent(".local/ui/assessment")
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-assessment-preview-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    let engine = root.appendingPathComponent("target/release/rove")
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = 1000
    settings.sampleEvery = 10
    let directory = output.appendingPathComponent("run-\(UUID())")
    let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings)
    #expect(
      await RunProcess().execute(
        RunRequest(id: UUID(), executable: engine, directory: directory, draft: draft)
      ) { _ in }.status == .completed)
    let entry = HistoryEntry.read(directory)
    let workspace = Workspace(executable: engine, preferences: preferences)
    let predictive = workspace.predictiveCheck(for: entry)
    predictive.replicates = "30"
    predictive.start()
    await predictive.waitForExit()
    let saved = try #require(predictive.predictions.first)
    let review = workspace.runAssessment(for: entry)
    review.start()
    await review.waitForExit()
    #expect(review.error == nil, Comment(rawValue: review.details ?? ""))
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      try await render(
        RunAssessmentView(workspace: workspace, entry: entry), name: "review", height: 950)
      try await render(
        PredictiveReportView(prediction: saved, strings: preferences.strings), name: "prediction",
        height: 850)

      @MainActor func render<V: View>(_ content: V, name: String, height: CGFloat) async throws {
        let view = NSHostingView(
          rootView: content.padding(24).background(Lab.background)
            .foregroundStyle(Lab.ink).frame(width: 820, height: height, alignment: .topLeading))
        let window = NSWindow(
          contentRect: NSRect(x: 0, y: 0, width: 820, height: height), styleMask: [.borderless],
          backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = view
        window.orderBack(nil)
        try await Task.sleep(for: .milliseconds(500))
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
