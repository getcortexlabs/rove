import AppKit
import SwiftUI

@main
struct RoveApp: App {
  @State private var workspace: Workspace
  @State private var access: BetaAccess
  @State private var experience = LaunchExperience()
  @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
  @Environment(\.openWindow) private var openWindow
  private var strings: AppStrings { workspace.strings }

  init() {
    Lab.registerFonts()
    let access = BetaAccess()
    _access = State(initialValue: access)
    _workspace = State(
      initialValue: Workspace(accessAllowed: { [weak access] in access?.canUse == true }))
  }

  var body: some Scene {
    Window("Rove", id: "workspace") {
      EntryView(workspace: workspace, access: access, experience: experience)
        .ignoresSafeArea(.container, edges: .top)
        .background(WindowSetup(workspace: workspace))
        .onOpenURL {
          if access.canUse && experience.ready {
            workspace.open($0)
          } else {
            experience.pendingURL = $0
          }
        }
        .onAppear { delegate.workspace = workspace }
    }
    .windowStyle(.hiddenTitleBar)
    .defaultSize(width: 1240, height: 850)
    .windowResizability(.contentMinSize)
    .commands {
      CommandGroup(replacing: .appInfo) {
        Button(strings("About Rove")) { openWindow(id: "about") }
      }
      CommandGroup(replacing: .appSettings) {
        SettingsLink { Text(strings("Settings…")) }.keyboardShortcut(",", modifiers: .command)
      }
      CommandGroup(replacing: .newItem) {
        Button(strings("New Analysis"), action: workspace.reset).keyboardShortcut("n").disabled(
          workspace.isRunning || !access.canUse || !experience.ready)
        Button(strings("Open Matrix…"), action: workspace.chooseFile).keyboardShortcut("o")
          .disabled(workspace.isRunning || !access.canUse || !experience.ready)
        Button(strings("Open Setup…"), action: workspace.chooseSetup).keyboardShortcut(
          "o", modifiers: [.command, .shift]
        ).disabled(workspace.isRunning || !access.canUse || !experience.ready)
        Button(strings("Try Example Matrix"), action: workspace.loadExample).disabled(
          workspace.isRunning || !access.canUse || !experience.ready)
        Button(strings("Summarize Tree File…"), action: workspace.summarizeTreeFile)
          .keyboardShortcut("t", modifiers: [.command, .shift])
          .disabled(
            workspace.isRunning || workspace.loading || workspace.treeSummary != nil
              || !access.canUse || !experience.ready)
        Divider()
        Button(strings("Save Setup…"), action: workspace.saveDraft).keyboardShortcut("s")
          .disabled(
            !access.canUse || !experience.ready || workspace.report == nil || workspace.issue != nil
              || workspace.isRunning
              || workspace.showingHistory)
        Divider()
        Button(strings("Start analysis"), action: workspace.startRun).keyboardShortcut(
          .return, modifiers: .command
        ).disabled(!workspace.canStartRun || !experience.ready)
        Divider()
        Button(strings("History"), action: workspace.openHistory).keyboardShortcut(
          "h", modifiers: [.command, .shift]
        ).disabled(!access.canUse || !experience.ready)
        Button(strings("Back to workspace"), action: workspace.returnToWorkspace).keyboardShortcut(
          "r", modifiers: [.command, .shift]
        ).disabled(!workspace.showingHistory || !access.canUse || !experience.ready)
      }
      CommandGroup(replacing: .help) {
        Button(strings("Benchmark this Mac")) { openWindow(id: "benchmark") }
          .disabled(!workspace.canOpenBenchmark || !experience.ready)
        Divider()
        Button(strings("Take the Rove tour")) { experience.presentation = .tour }
          .disabled(
            !access.canUse || !experience.ready || workspace.isRunning || workspace.loading
              || workspace.treeSummary != nil)
        Button(strings("What's new in Rove")) { experience.presentation = .news }
          .disabled(
            !access.canUse || !experience.ready || workspace.isRunning || workspace.loading
              || workspace.treeSummary != nil || experience.availableReleases.isEmpty)
        Divider()
        Button(strings("Rove Field Guide")) {
          NSWorkspace.shared.open(strings.guideURL)
        }
        Divider()
        if let session = access.session {
          Text(session.email)
          Button(strings("Sign out of the beta")) { access.signOut() }
            .disabled(workspace.isRunning)
        }
      }
    }
    Window(strings("About Rove"), id: "about") {
      AboutView(preferences: workspace.preferences)
    }
    .windowStyle(.hiddenTitleBar)
    .windowResizability(.contentSize)
    .defaultPosition(.center)
    .commandsRemoved()

    Window(strings("Benchmark this Mac"), id: "benchmark") {
      BenchmarkView(workspace: workspace, access: access)
    }
    .defaultSize(width: 740, height: 820)
    .windowResizability(.contentMinSize)

    Window(strings("Tree explorer"), id: "tree-explorer") {
      if let model = workspace.treeExplorer {
        TreeExplorerView(model: model, preferences: workspace.preferences).id(model.id)
      }
    }
    .defaultSize(width: 1100, height: 780)
    .windowResizability(.contentMinSize)

    Window(strings("Trace diagnostics"), id: "trace-diagnostics") {
      if let model = workspace.traceDiagnostics {
        TraceDiagnosticsView(model: model, preferences: workspace.preferences) { directory in
          workspace.openTree(directory, run: model.directory)
          openWindow(id: "tree-explorer")
        }.id(model.id)
      }
    }
    .defaultSize(width: 1160, height: 850)
    .windowResizability(.contentMinSize)

    Settings {
      SettingsView(preferences: workspace.preferences)
    }.windowResizability(.contentSize)
  }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
  var workspace: Workspace?
  private var quitting = false

  func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
    guard let workspace, workspace.isRunning else { return .terminateNow }
    guard !quitting, confirmStop(workspace) else { return .terminateCancel }
    quitting = true
    Task {
      await workspace.cancelAndWait()
      sender.reply(toApplicationShouldTerminate: true)
    }
    return .terminateLater
  }
  func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}

struct WindowSetup: NSViewRepresentable {
  let workspace: Workspace

  func makeCoordinator() -> Coordinator { Coordinator(workspace: workspace) }

  final class Coordinator: NSObject, NSWindowDelegate {
    let workspace: Workspace
    var closing = false
    private let confirm: @MainActor () -> Bool
    init(workspace: Workspace, confirm: (@MainActor () -> Bool)? = nil) {
      self.workspace = workspace
      self.confirm = confirm ?? { confirmStop(workspace) }
    }
    func windowShouldClose(_ sender: NSWindow) -> Bool {
      guard workspace.isRunning else { return true }
      guard !closing, confirm() else { return false }
      closing = true
      Task { @MainActor in
        await workspace.cancelAndWait()
        closing = false
        sender.performClose(nil)
      }
      return false
    }
  }

  func makeNSView(context: Context) -> NSView {
    let view = NSView()
    Task { @MainActor in
      guard let window = view.window else { return }
      window.delegate = context.coordinator
      window.titlebarAppearsTransparent = true
      window.backgroundColor = NSColor(srgbRed: 16 / 255, green: 17 / 255, blue: 23 / 255, alpha: 1)
      window.isMovableByWindowBackground = false
      window.appearance = NSAppearance(named: .darkAqua)
      #if DEBUG
        await captureIfRequested(window)
      #endif
    }
    return view
  }

  func updateNSView(_ nsView: NSView, context: Context) {}

  #if DEBUG
    @MainActor private func captureIfRequested(_ window: NSWindow) async {
      let arguments = ProcessInfo.processInfo.arguments
      guard let index = arguments.firstIndex(of: "--snapshot"),
        arguments.indices.contains(index + 1)
      else { return }
      if arguments.contains("--example") {
        workspace.loadExample()
        while workspace.loading { try? await Task.sleep(for: .milliseconds(50)) }
      }
      if arguments.contains("--model") { workspace.page = .model }
      if arguments.contains("--review") { workspace.page = .review }
      if arguments.contains("--compact") { window.setContentSize(NSSize(width: 1080, height: 700)) }
      try? await Task.sleep(for: .milliseconds(700))
      guard let content = window.contentView,
        let bitmap = content.bitmapImageRepForCachingDisplay(in: content.bounds)
      else { return }
      content.cacheDisplay(in: content.bounds, to: bitmap)
      if let png = bitmap.representation(using: .png, properties: [:]) {
        try? png.write(to: URL(fileURLWithPath: arguments[index + 1]))
      }
      NSApp.terminate(nil)
    }
  #endif
}

@MainActor private func confirmStop(_ workspace: Workspace) -> Bool {
  let strings = workspace.strings
  let alert = NSAlert()
  alert.messageText = strings(
    workspace.benchmark.isRunning
      ? "A benchmark is still running." : "An analysis is still running.")
  alert.informativeText = strings(
    workspace.benchmark.isRunning
      ? "Closing Rove will cancel this benchmark. Incomplete tests cannot be shared."
      : "Closing Rove will stop the engine. History can recover the latest saved checkpoint. To save the current point, keep Rove open and pause the analysis first."
  )
  alert.addButton(withTitle: strings("Keep running"))
  alert.addButton(withTitle: strings("Stop and close"))
  return alert.runModal() == .alertSecondButtonReturn
}
