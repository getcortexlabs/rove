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
    let diagnostics = TraceDiagnosticsModel(
      directory: run.request.directory,
      executable: root.appendingPathComponent("target/release/rove"))
    diagnostics.burnin = "25"
    diagnostics.start()
    await diagnostics.waitForExit()
    #expect(diagnostics.error == nil)
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      for size in [NSSize(width: 1160, height: 850), NSSize(width: 960, height: 700)] {
        let view = NSHostingView(
          rootView: TraceDiagnosticsView(
            model: diagnostics, preferences: preferences, onOpenTree: { _ in }))
        let window = NSWindow(
          contentRect: NSRect(origin: .zero, size: size), styleMask: [.borderless],
          backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.appearance = NSAppearance(named: .darkAqua)
        window.contentView = view
        window.orderBack(nil)
        try await capture(
          view,
          to: directory.appendingPathComponent(
            "diagnostics-\(language.rawValue)-\(Int(size.width)).png"))
        window.close()
      }
    }
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
        rootView: TreeSummaryView(model: summary, strings: workspace.strings, onOpen: { _ in }))
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

    let saved = try #require(summary.summaries.first)
    let explorer = TreeExplorer(
      directory: saved.directory, runDirectory: run.request.directory,
      executable: root.appendingPathComponent("target/release/rove"))
    await explorer.load()
    #expect(explorer.error == nil)
    explorer.selected = explorer.document?.nodes.first { $0.children != nil }?.id
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      for size in [NSSize(width: 1100, height: 780), NSSize(width: 880, height: 640)] {
        let view = NSHostingView(
          rootView: TreeExplorerView(model: explorer, preferences: preferences))
        let window = NSWindow(
          contentRect: NSRect(origin: .zero, size: size), styleMask: [.borderless],
          backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = view
        window.appearance = NSAppearance(named: .darkAqua)
        window.orderBack(nil)
        try await capture(
          view,
          to: directory.appendingPathComponent("tree-\(language.rawValue)-\(Int(size.width)).png"))
        window.close()
      }
    }
    let drawing = TreeDrawing()
    drawing.model = explorer
    let tree = try #require(explorer.document)
    drawing.figure = TreeFigure(
      tree: tree, layout: try #require(explorer.layout),
      viewport: CGSize(width: 760, height: 600), collapsed: [], selected: tree.root,
      showSupport: true, strings: preferences.strings)
    explorer.selected = tree.root
    for (key, expected) in [
      (UInt16(124), tree.nodes[tree.root].children![0]), (UInt16(123), tree.root),
    ] {
      let event = try #require(
        NSEvent.keyEvent(
          with: .keyDown, location: .zero, modifierFlags: [],
          timestamp: 0, windowNumber: 0, context: nil, characters: "",
          charactersIgnoringModifiers: "",
          isARepeat: false, keyCode: key))
      drawing.keyDown(with: event)
      #expect(explorer.selected == expected)
    }
    #expect(drawing.accessibilityLabel() == preferences.strings("Phylogenetic tree"))
    explorer.selected = tree.nodes.first { $0.children != nil }?.id
    explorer.toggleCollapse()
    await explorer.waitForLayout()
    explorer.rotate()
    await explorer.waitForLayout()
    let figure = TreeFigure(
      tree: try #require(explorer.document), layout: try #require(explorer.layout),
      viewport: CGSize(width: 760, height: 600), collapsed: explorer.collapsed,
      selected: explorer.selected, showSupport: true, strings: preferences.strings)
    try figure.write(
      to: directory.appendingPathComponent("tree-collapsed.svg"), runName: "Preview",
      protectedFiles: [])

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
        await worker.execute(request) { event in
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

  @Test @MainActor func renderParallelChains() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let output = root.appendingPathComponent(".local/ui/parallel")
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-parallel-preview-\(UUID())"
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
    workspace.page = .model
    workspace.settings.chains = 4
    workspace.settings.steps = 200_000
    workspace.settings.sampleEvery = 100
    let session = output.appendingPathComponent("session-\(UUID())")
    let request = RunRequest(
      id: UUID(), executable: engine, directory: session,
      draft: AnalysisDraft(
        matrix: matrix, report: report, settings: workspace.settings, outgroup: "T20"))
    let result = await RunProcess().execute(request) { _ in }
    #expect(result.status == .completed)
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      for kind in ["setup", "comparison"] {
        let content =
          kind == "setup"
          ? AnyView(RootView(workspace: workspace))
          : AnyView(
            ScrollView {
              ChainResultsView(
                workspace: workspace, directory: session, settings: workspace.settings,
                completed: true
              )
              .padding(28)
            }.background(Lab.background).foregroundStyle(Lab.ink))
        let view = NSHostingView(rootView: content)
        let window = NSWindow(
          contentRect: NSRect(x: 0, y: 0, width: 1240, height: kind == "setup" ? 1650 : 950),
          styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = view
        window.orderBack(nil)
        try await capture(
          view, to: output.appendingPathComponent("\(kind)-\(language.rawValue).png"))
        window.close()
      }
    }
  }

  @Test @MainActor func renderPausedHistory() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent()
    let output = root.appendingPathComponent(".local/ui/checkpoint")
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-checkpoint-preview-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    let history = RunHistory(defaults: defaults, defaultRoot: output)
    let engine = root.appendingPathComponent("target/release/rove")
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    let workspace = Workspace(executable: engine, preferences: preferences, history: history)
    workspace.page = .review
    workspace.report = report
    workspace.file = matrix
    workspace.outgroup = "T20"
    workspace.settings.chains = 4
    workspace.settings.steps = 10_000_000
    workspace.settings.sampleEvery = 1_000
    workspace.outputParent = output
    workspace.startRun()
    let run = try #require(workspace.run)
    for _ in 0..<300 {
      if run.completedSteps >= 100_000 || !run.isActive { break }
      try await Task.sleep(for: .milliseconds(20))
    }
    run.pause()
    await run.waitForExit()
    #expect(run.status == .paused)
    await history.reload()
    workspace.showingHistory = true
    workspace.selectedHistoryID = run.request.directory.path
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      let view = NSHostingView(rootView: RootView(workspace: workspace))
      let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: 1240, height: 1550),
        styleMask: [.borderless], backing: .buffered, defer: false)
      window.isReleasedWhenClosed = false
      window.contentView = view
      window.orderBack(nil)
      try await capture(view, to: output.appendingPathComponent("paused-\(language.rawValue).png"))
      window.close()
    }
    workspace.resumeRun(HistoryEntry.read(run.request.directory))
    let resumed = try #require(workspace.run)
    #expect(resumed.request.resume)
    resumed.pause()
    await resumed.waitForExit()
    #expect(resumed.status == .paused)
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
