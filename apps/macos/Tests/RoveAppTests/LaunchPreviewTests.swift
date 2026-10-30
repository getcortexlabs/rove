import AppKit
import SwiftUI
import Testing

@testable import RoveApp

struct LaunchPreviewTests {
  @Test @MainActor func renderEntryAndWelcome() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let directory = root.appendingPathComponent(".local/ui/launch")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    _ = NSApplication.shared
    NSApp.appearance = NSAppearance(named: .darkAqua)
    Lab.registerFonts()
    let suite = "rove-launch-preview-\(UUID().uuidString)"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults)
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      let access = BetaAccess(
        client: TestBetaClient(session: BetaAccessTests.session(at: Date())),
        store: MemoryBetaStore(), now: Date.init)
      await access.restore()
      for size in [NSSize(width: 1080, height: 700), NSSize(width: 1240, height: 850)] {
        try await capture(
          DedicationView(preferences: preferences, onContinue: {}), size: size,
          to: directory.appendingPathComponent(
            "\(language.rawValue)-dedication-\(Int(size.height)).png"))
      }
      for name in ["invitation", "waitlist", "code", "confirmed"] {
        if name == "waitlist" { access.begin(.waitlist) }
        if name == "code" {
          await access.request(email: "explorer@example.com", language: language.rawValue)
        }
        if name == "confirmed" { await access.verify(code: "12345678") }
        try await capture(
          BetaEntryView(access: access, preferences: preferences),
          size: NSSize(width: 1080, height: 700),
          to: directory.appendingPathComponent("\(language.rawValue)-\(name).png"))
      }
      for step in 0..<4 {
        try await capture(
          OnboardingView(preferences: preferences, onFinish: { _ in }, step: step),
          size: NSSize(width: 1080, height: 700),
          to: directory.appendingPathComponent("\(language.rawValue)-tour-\(step).png"))
      }
      try await capture(
        WhatsNewView(
          preferences: preferences, releases: RoveRelease.all,
          initial: RoveRelease.all[0], onDone: {}, onAction: { _ in }),
        size: NSSize(width: 700, height: 740),
        to: directory.appendingPathComponent("\(language.rawValue)-news.png"))
    }
  }

  @MainActor private func capture<V: View>(_ content: V, size: NSSize, to url: URL) async throws {
    let view = NSHostingView(
      rootView: content.environment(
        \.visualComfort, VisualComfort(reduceMotion: true, reduceTransparency: true)
      )
      .background { WorkspaceBackdrop() }.foregroundStyle(Lab.ink).preferredColorScheme(.dark))
    view.frame = NSRect(origin: .zero, size: size)
    let window = NSWindow(
      contentRect: view.frame, styleMask: [.borderless], backing: .buffered, defer: false)
    window.contentView = view
    window.setContentSize(size)
    defer { window.orderOut(nil) }
    try await Task.sleep(for: .milliseconds(200))
    view.layoutSubtreeIfNeeded()
    let bitmap = try #require(view.bitmapImageRepForCachingDisplay(in: view.bounds))
    view.cacheDisplay(in: view.bounds, to: bitmap)
    try #require(bitmap.representation(using: .png, properties: [:])).write(to: url)
  }
}
