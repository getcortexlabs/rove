import AppKit
import SwiftUI

@main
struct RoveApp: App {
  @State private var workspace = Workspace()
  @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
  private var strings: AppStrings { workspace.strings }

  init() { Lab.registerFonts() }

  var body: some Scene {
    Window("Rove", id: "workspace") {
      RootView(workspace: workspace)
        .ignoresSafeArea(.container, edges: .top)
        .background(WindowSetup(workspace: workspace))
        .onOpenURL { workspace.load($0) }
    }
    .windowStyle(.hiddenTitleBar)
    .defaultSize(width: 1240, height: 850)
    .windowResizability(.contentMinSize)
    .commands {
      CommandGroup(replacing: .appSettings) {
        SettingsLink { Text(strings("Settings…")) }.keyboardShortcut(",", modifiers: .command)
      }
      CommandGroup(replacing: .newItem) {
        Button(strings("New Analysis"), action: workspace.reset).keyboardShortcut("n")
        Button(strings("Open Matrix…"), action: workspace.chooseFile).keyboardShortcut("o")
        Button(strings("Try Example Matrix"), action: workspace.loadExample)
        Divider()
        Button(strings("Save Setup…"), action: workspace.saveDraft).keyboardShortcut("s")
          .disabled(workspace.report == nil || workspace.issue != nil)
      }
      CommandGroup(replacing: .help) {
        Button(strings("Rove Field Guide")) {
          NSWorkspace.shared.open(strings.guideURL)
        }
      }
    }
    Settings {
      SettingsView(preferences: workspace.preferences)
    }.windowResizability(.contentSize)
  }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
  func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}

private struct WindowSetup: NSViewRepresentable {
  let workspace: Workspace

  func makeNSView(context: Context) -> NSView {
    let view = NSView()
    Task { @MainActor in
      guard let window = view.window else { return }
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
