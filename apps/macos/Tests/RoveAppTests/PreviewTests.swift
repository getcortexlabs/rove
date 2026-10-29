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
    let directory = root.appendingPathComponent(".local/ui")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    _ = NSApplication.shared
    Lab.registerFonts()
    let workspace = Workspace()
    let report = try await MatrixInspector.inspect(
      root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"), gapsAsMissing: false,
      executable: root.appendingPathComponent("target/release/rove"))
    let states: [(String, WorkspacePage?, CGFloat, CGFloat)] = [
      ("welcome", nil, 1240, 850), ("matrix", .matrix, 1240, 850),
      ("model", .model, 1240, 850), ("review", .review, 1240, 850),
      ("welcome-compact", nil, 1080, 700), ("model-compact", .model, 1080, 700),
    ]
    for (name, page, width, height) in states {
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
      window.contentView = view
      window.orderBack(nil)
      try? await Task.sleep(for: .milliseconds(300))
      view.layoutSubtreeIfNeeded()
      let bitmap = try #require(view.bitmapImageRepForCachingDisplay(in: view.bounds))
      view.cacheDisplay(in: view.bounds, to: bitmap)
      let png = try #require(bitmap.representation(using: .png, properties: [:]))
      try png.write(to: directory.appendingPathComponent("\(name).png"))
      window.close()
    }
  }
}
