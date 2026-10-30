import AppKit
import SwiftUI
import Testing

@testable import RoveApp

private actor BenchmarkUploadStub: BenchmarkSharing {
  var attempts = 0
  var received: [Data] = []
  func upload(_ data: Data, token: String, id: UUID) async throws {
    attempts += 1
    received.append(data)
    if attempts == 1 { throw BenchmarkError.unavailable }
  }
}

struct BenchmarkTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private func folder() throws -> URL {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-benchmark-\(UUID())")
    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    return url
  }

  private func record() throws -> BenchmarkRecord {
    let data = try BenchmarkProtocol.data
    let p = try JSONDecoder().decode(BenchmarkProtocol.self, from: data)
    var payload = BenchmarkPayload(
      id: UUID(), createdAt: Date().timeIntervalSince1970,
      appVersion: "0.1.0", appBuild: "1", appSHA256: String(repeating: "a", count: 64),
      hardware: .current(), before: .current())
    payload.after = .current()
    payload.protocol = p
    payload.protocolSHA256 = benchmarkHash(data)
    payload.engine = BenchmarkEngine(
      version: "0.1.0", sha256: String(repeating: "b", count: 64),
      architecture: "aarch64", os: "macos", optimized: true, experimentalRepeats: false,
      backend: "neon")
    payload.samples = [1, 4].flatMap { chains in
      (0..<3).map { repetition in
        BenchmarkSample(
          chains: chains, repetition: repetition, seconds: Double(2 + repetition) / 2,
          checksums: (0..<chains).map { String(repeating: String($0 + 1), count: 64) })
      }
    }
    return BenchmarkRecord(payload: payload, status: "completed")
  }

  @Test @MainActor func persistenceAndExplicitSharingSurviveFailureAndRetry() async throws {
    let directory = try folder()
    defer { try? FileManager.default.removeItem(at: directory) }
    let store = BenchmarkStore(directory: directory)
    let first = try record()
    let second = try record()
    try store.save(first)
    try store.save(second)
    let upload = BenchmarkUploadStub()
    let model = Benchmark(store: store, client: upload)
    #expect(model.records.count == 2)
    #expect(await upload.attempts == 0)
    model.selectedID = first.id
    await model.share(token: nil)
    #expect(await upload.attempts == 0)
    await model.share(token: "test-token")
    #expect(model.error == BenchmarkError.unavailable.message)
    #expect(model.selected?.sharedAt == nil)
    #expect(try store.load().first(where: { $0.id == first.id })?.status == "completed")
    await model.share(token: "test-token")
    #expect(model.selected?.sharedAt != nil)
    #expect(model.error == nil)
    #expect(await upload.attempts == 2)
    let reloaded = try #require(Benchmark(store: store).records.first(where: { $0.id == first.id }))
    let payload = try JSONDecoder().decode(BenchmarkPayload.self, from: reloaded.submission())
    #expect(payload.id == first.id)
    #expect(payload.samples == first.payload.samples)
    await model.share(token: "test-token")
    #expect(await upload.attempts == 2)
    let bytes = try first.submission()
    let json = try #require(JSONSerialization.jsonObject(with: bytes) as? [String: Any])
    #expect(
      Set(json.keys)
        == Set([
          "schemaVersion", "id", "createdAt", "appVersion", "appBuild", "appSHA256", "hardware",
          "before", "after", "protocol", "protocolSHA256", "engine", "samples",
        ]))
  }

  @Test func incompleteAndCorruptRecordsCannotBeShared() throws {
    let directory = try folder()
    defer { try? FileManager.default.removeItem(at: directory) }
    let store = BenchmarkStore(directory: directory)
    var interrupted = try record()
    interrupted.status = "running"
    try store.save(interrupted)
    let restored = try #require(store.load().first)
    #expect(restored.status == "interrupted")
    #expect(throws: BenchmarkError.self) { try restored.submission() }
    var partial = try record()
    partial.payload.samples.removeLast()
    #expect(throws: BenchmarkError.self) { try partial.submission() }
    partial = try record()
    partial.payload.samples.reverse()
    #expect(throws: BenchmarkError.self) { try partial.submission() }
    partial = try record()
    partial.payload.protocolSHA256 = String(repeating: "0", count: 64)
    #expect(throws: BenchmarkError.self) { try partial.submission() }
    try Data("broken".utf8).write(to: directory.appendingPathComponent("broken.json"))
    #expect(try store.load().count == 1)
  }

  @Test @MainActor func shippedEngineCompletesOfflineAndCancellationBlocksAnalysisStart()
    async throws
  {
    let directory = try folder()
    defer { try? FileManager.default.removeItem(at: directory) }
    let upload = BenchmarkUploadStub()
    let model = Benchmark(store: BenchmarkStore(directory: directory), client: upload)
    let executable = root.appendingPathComponent("target/release/rove")
    model.start(executable: executable)
    await model.waitForExit()
    let saved = try #require(model.selected)
    #expect(saved.status == "completed")
    #expect(saved.payload.samples.count == 6)
    #expect(model.error == nil)
    #expect(await upload.attempts == 0)
    _ = try saved.submission()
    let output = root.appendingPathComponent(".local/issue39")
    if FileManager.default.fileExists(atPath: output.path) {
      try saved.submission().write(to: output.appendingPathComponent("native-payload.json"))
    }
    let suite = "rove-benchmark-test-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let workspace = Workspace(
      executable: executable, preferences: AppPreferences(defaults: defaults),
      history: RunHistory(defaults: defaults, defaultRoot: directory), benchmark: model)
    workspace.file = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    workspace.report = try await MatrixInspector.inspect(
      workspace.file!, gapsAsMissing: false, executable: executable)
    workspace.page = .review
    #expect(workspace.canStartRun)
    workspace.startBenchmark()
    #expect(workspace.isRunning)
    #expect(!workspace.canStartRun)
    workspace.startRun()
    #expect(workspace.run == nil)
    await workspace.cancelAndWait()
    #expect(!workspace.isRunning)
    #expect(model.selected?.status == "cancelled")
    #expect(model.records.count == 2)
    let stopped = try #require(model.selected)
    #expect(throws: BenchmarkError.self) { try stopped.submission() }
  }

  @Test @MainActor func renderBenchmarkResults() async throws {
    guard ProcessInfo.processInfo.environment["CI"] == nil else { return }
    let directory = try folder()
    defer { try? FileManager.default.removeItem(at: directory) }
    let store = BenchmarkStore(directory: directory)
    try store.save(record())
    let suite = "rove-benchmark-preview-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    _ = NSApplication.shared
    Lab.registerFonts()
    let preferences = AppPreferences(defaults: defaults)
    let workspace = Workspace(preferences: preferences, benchmark: Benchmark(store: store))
    let output = root.appendingPathComponent(".local/ui/benchmark")
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    for language in [AppLanguage.english, .portuguese] {
      preferences.language = language
      let view = NSHostingView(rootView: BenchmarkView(workspace: workspace, access: BetaAccess()))
      let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: 760, height: 1140), styleMask: [.borderless],
        backing: .buffered, defer: false)
      window.isReleasedWhenClosed = false
      window.appearance = NSAppearance(named: .darkAqua)
      window.contentView = view
      window.orderBack(nil)
      try await Task.sleep(for: .milliseconds(350))
      view.layoutSubtreeIfNeeded()
      let bitmap = try #require(view.bitmapImageRepForCachingDisplay(in: view.bounds))
      view.cacheDisplay(in: view.bounds, to: bitmap)
      try #require(bitmap.representation(using: .png, properties: [:])).write(
        to: output.appendingPathComponent("result-\(language.rawValue).png"))
      window.close()
    }
  }
}
