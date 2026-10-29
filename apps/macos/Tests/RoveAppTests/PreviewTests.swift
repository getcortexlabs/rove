import AppKit
import SwiftUI
import Testing

@testable import RoveApp

struct PreviewTests {
  @Test @MainActor func renderWorkspaceStates() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let package = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent()
    let root = package.deletingLastPathComponent().deletingLastPathComponent()
    let directory = root.appendingPathComponent(".local/ui/i18n")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-preview-\(UUID().uuidString)"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    let historyRoot = directory.appendingPathComponent("history-\(UUID().uuidString)")
    let history = RunHistory(defaults: defaults, defaultRoot: historyRoot)
    let workspace = Workspace(
      executable: root.appendingPathComponent("target/release/rove"), preferences: preferences,
      history: history)
    let report = try await MatrixInspector.inspect(
      root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"), gapsAsMissing: false,
      executable: root.appendingPathComponent("target/release/rove"))
    let states: [(String, WorkspacePage?, CGFloat, CGFloat)] = [
      ("welcome", nil, 1240, 850), ("matrix", .matrix, 1240, 850),
      ("model", .model, 1240, 850), ("review", .review, 1240, 850),
      ("welcome-compact", nil, 1080, 700), ("model-compact", .model, 1080, 700),
      ("outgroup-model", .model, 1240, 850), ("outgroup-review", .review, 1240, 850),
      ("outgroup-model-compact", .model, 1080, 700),
      ("outgroup-review-compact", .review, 1080, 700),
      ("history-empty", nil, 1240, 850), ("history-empty-compact", nil, 1080, 700),
    ]
    for (name, page, width, height) in states {
      workspace.showingHistory = name.hasPrefix("history")
      workspace.outgroup = name.hasPrefix("outgroup") ? "T20" : nil
      workspace.report = page == nil ? nil : report
      workspace.file =
        page == nil ? nil : root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
      workspace.isExample = page != nil
      workspace.page = page ?? .matrix
      let view = NSHostingView(rootView: RootView(workspace: workspace))
      let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: width, height: height),
        styleMask: [.borderless], backing: .buffered, defer: false)
      window.isReleasedWhenClosed = false
      window.appearance = NSAppearance(named: .darkAqua)
      window.contentView = view
      window.orderBack(nil)
      for language in [AppLanguage.english, .portuguese] {
        preferences.language = language
        try await capture(
          view, to: directory.appendingPathComponent("\(name)-\(language.rawValue).png"))
      }
      window.close()
    }
    workspace.showingHistory = false
    workspace.report = report
    workspace.file = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    workspace.page = .review
    workspace.settings.steps = 1_000_000
    workspace.outputParent = historyRoot
    workspace.outgroup = "T20"
    let editor = NSHostingView(
      rootView: GroupEditor(
        workspace: workspace,
        selection: GroupSelection(group: Array(report.allTaxa.prefix(19)))))
    let editorWindow = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 526, height: 650),
      styleMask: [.borderless], backing: .buffered, defer: false)
    editorWindow.isReleasedWhenClosed = false
    editorWindow.appearance = NSAppearance(named: .darkAqua)
    editorWindow.contentView = editor
    editorWindow.orderBack(nil)
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      try await capture(
        editor, to: directory.appendingPathComponent("group-editor-\(language.rawValue).png"))
    }
    editorWindow.close()
    workspace.startRun()
    let run = try #require(workspace.run)
    let runView = NSHostingView(rootView: RootView(workspace: workspace))
    let runWindow = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 1240, height: 850),
      styleMask: [.borderless], backing: .buffered, defer: false)
    runWindow.isReleasedWhenClosed = false
    runWindow.appearance = NSAppearance(named: .darkAqua)
    runWindow.contentView = runView
    runWindow.orderBack(nil)
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      try await capture(
        runView, to: directory.appendingPathComponent("running-\(language.rawValue).png"))
    }
    await run.waitForExit()
    #expect(run.status == .completed)
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      try await capture(
        runView, to: directory.appendingPathComponent("completed-\(language.rawValue).png"))
    }
    runWindow.close()
    let summary = TreeSummaryModel(
      directory: run.request.directory,
      executable: root.appendingPathComponent("target/release/rove"))
    summary.burnin = "25"
    summary.start()
    await summary.waitForExit()
    #expect(summary.error == nil && summary.summaries.count == 1)
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      let view = NSHostingView(
        rootView: TreeSummaryView(model: summary, strings: workspace.strings))
      let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: 620, height: 650), styleMask: [.borderless],
        backing: .buffered, defer: false)
      window.isReleasedWhenClosed = false
      window.appearance = NSAppearance(named: .darkAqua)
      window.contentView = view
      window.orderBack(nil)
      try await capture(
        view, to: directory.appendingPathComponent("summary-\(language.rawValue).png"))
      window.close()
    }

    for (name, seed, cancelled) in [("failed", "43", false), ("cancelled", "44", true)] {
      var configuration = workspace.settings
      configuration.seed = seed
      configuration.steps = 100_000_000
      let request = RunRequest(
        id: UUID(),
        executable: cancelled
          ? root.appendingPathComponent("target/release/rove")
          : historyRoot.appendingPathComponent("missing-engine"),
        directory: historyRoot.appendingPathComponent(name),
        draft: AnalysisDraft(
          matrix: workspace.file!, report: report, settings: configuration,
          monophyly: workspace.monophyly, outgroup: workspace.outgroup))
      let worker = RunProcess()
      _ = await Task.detached {
        worker.execute(request) { event in
          if cancelled && event.event == "started" { worker.cancel() }
        }
      }.value
    }
    await history.reload()
    workspace.showingHistory = true
    for (name, selected, width, height) in [
      ("history", false, 1240.0, 850.0), ("history-compact", false, 1080.0, 700.0),
      ("history-detail", true, 1240.0, 850.0), ("history-detail-compact", true, 1080.0, 700.0),
    ] {
      workspace.selectedHistoryID = selected ? run.request.directory.path : nil
      let view = NSHostingView(rootView: RootView(workspace: workspace))
      let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: width, height: height),
        styleMask: [.borderless], backing: .buffered, defer: false)
      window.isReleasedWhenClosed = false
      window.appearance = NSAppearance(named: .darkAqua)
      window.contentView = view
      window.orderBack(nil)
      for language in [AppLanguage.english, .portuguese] {
        preferences.language = language
        try await capture(
          view, to: directory.appendingPathComponent("\(name)-\(language.rawValue).png"))
      }
      window.close()
    }
    workspace.showingHistory = false
    let settings = NSHostingView(rootView: SettingsView(preferences: preferences))
    let settingsWindow = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 560, height: 590),
      styleMask: [.borderless], backing: .buffered, defer: false)
    settingsWindow.isReleasedWhenClosed = false
    settingsWindow.appearance = NSAppearance(named: .darkAqua)
    settingsWindow.contentView = settings
    settingsWindow.orderBack(nil)
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      try await capture(
        settings, to: directory.appendingPathComponent("settings-\(language.rawValue).png"))
    }
    settingsWindow.close()
    let app = Bundle(url: root.appendingPathComponent("build/macos/Rove.app"))
    let version = app?.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      for solid in [false, true] {
        preferences.reduceTransparency = solid
        let view = NSHostingView(rootView: AboutView(preferences: preferences, version: version))
        let window = NSWindow(
          contentRect: NSRect(origin: .zero, size: view.fittingSize),
          styleMask: [.titled, .closable, .fullSizeContentView], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.appearance = NSAppearance(named: .darkAqua)
        window.contentView = view
        window.orderBack(nil)
        try await capture(
          view,
          to: directory.appendingPathComponent(
            "about-\(language.rawValue)\(solid ? "-solid" : "").png"))
        window.close()
      }
    }

  }

  @MainActor private func capture(_ view: NSView, to url: URL) async throws {
    try await Task.sleep(for: .milliseconds(350))
    view.layoutSubtreeIfNeeded()
    let bitmap = try #require(view.bitmapImageRepForCachingDisplay(in: view.bounds))
    view.cacheDisplay(in: view.bounds, to: bitmap)
    let png = try #require(bitmap.representation(using: .png, properties: [:]))
    try png.write(to: url)
  }
}
