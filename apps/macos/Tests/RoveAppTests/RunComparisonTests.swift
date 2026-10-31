import Foundation
import Testing

@testable import RoveApp

struct RunComparisonTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }

  private func run(
    _ directory: URL, matrix: URL, seed: String, chains: Int = 1, steps: Int = 2_000
  ) async throws -> HistoryEntry {
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = steps
    settings.sampleEvery = 100
    settings.seed = seed
    settings.chains = chains
    let request = RunRequest(
      id: UUID(), executable: engine, directory: directory,
      draft: AnalysisDraft(
        matrix: matrix, report: report, settings: settings,
        monophyly: [Array(report.allTaxa.prefix(19))]))
    #expect(await RunProcess().execute(request) { _ in }.status == .completed)
    return HistoryEntry.read(directory)
  }

  @Test @MainActor func compatibleRunsCompareAndSaveTheReport() async throws {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-compare-\(UUID())")
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: folder) }
    let matrix = folder.appendingPathComponent("matrix.nex")
    try Data(contentsOf: root.appendingPathComponent("fixtures/reference/20x50/matrix.nex"))
      .write(to: matrix)
    let other = folder.appendingPathComponent("control.nex")
    try Data(contentsOf: root.appendingPathComponent("fixtures/synthetic/control_20x50.nex"))
      .write(to: other)
    let first = try await run(folder.appendingPathComponent("first"), matrix: matrix, seed: "42")
    let second = try await run(folder.appendingPathComponent("second"), matrix: matrix, seed: "43")
    let group = try await run(
      folder.appendingPathComponent("group"), matrix: matrix, seed: "50", chains: 2, steps: 1_000)
    let sameSeed = try await run(folder.appendingPathComponent("same"), matrix: matrix, seed: "42")
    let different = try await run(folder.appendingPathComponent("other"), matrix: other, seed: "7")

    #expect(RunComparisonModel.compatible(first, second))
    #expect(RunComparisonModel.compatible(first, group))
    #expect(!RunComparisonModel.compatible(first, first))
    #expect(!RunComparisonModel.compatible(first, sameSeed))
    #expect(!RunComparisonModel.compatible(first, different))
    #expect(RunComparisonModel.chainDirectories(group).count == 2)
    let overlapping = try await run(
      folder.appendingPathComponent("overlap"), matrix: matrix, seed: "51", steps: 1_000)
    #expect(RunComparisonModel.compatible(first, overlapping))
    let selection = RunComparisonModel(entry: first, executable: engine)
    let pool = [second, group, overlapping]
    #expect(pool.allSatisfy { selection.canSelect($0, in: pool) })
    selection.selected = [group.id]
    #expect(!selection.canSelect(overlapping, in: pool))
    #expect(selection.canSelect(second, in: pool) && selection.canSelect(group, in: pool))
    selection.selected = [group.id, overlapping.id]
    selection.start(with: pool)
    await selection.waitForExit()
    #expect(!selection.busy && selection.comparisons.isEmpty)
    // The engine accepts at most 16 chain directories; four-chain groups reach it quickly.
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    func fourChains(_ name: String, seed: String) -> HistoryEntry {
      var settings = AnalysisSettings()
      settings.chains = 4
      settings.seed = seed
      settings.sampleEvery = 100
      let draft = AnalysisDraft(
        matrix: matrix, report: report, settings: settings,
        monophyly: [Array(report.allTaxa.prefix(19))])
      return HistoryEntry(
        directory: folder.appendingPathComponent(name), status: .completed, draft: draft,
        startedAt: nil, elapsedSeconds: nil, completedSteps: nil, message: nil, files: [],
        folderAvailable: true)
    }
    let base = fourChains("base", seed: "100")
    let groups = [
      fourChains("a", seed: "200"), fourChains("b", seed: "300"), fourChains("c", seed: "400"),
      fourChains("d", seed: "500"),
    ]
    let capped = RunComparisonModel(entry: base, executable: engine)
    #expect(capped.candidates(in: groups).count == 4)
    capped.selected = Set(groups.prefix(3).map(\.id))
    #expect(!capped.canSelect(groups[3], in: groups))
    #expect(capped.canSelect(groups[2], in: groups))

    let model = RunComparisonModel(entry: first, executable: engine)
    let candidates = model.candidates(in: [first, second, group, sameSeed, different])
    #expect(Set(candidates.map(\.id)) == Set([second.id, group.id]))
    model.selected = [second.id, group.id]
    model.burnin = "20"
    model.start(with: candidates)
    await model.waitForExit()
    #expect(model.error == nil, Comment(rawValue: model.details ?? ""))
    let saved = try #require(model.comparisons.first)
    #expect(saved.report.chainCount == 4)
    #expect(saved.report.burninPercent == 20)
    #expect(saved.report.retainedPerChain == 9)
    #expect(saved.sources.count == 4)
    #expect(saved.sources[0].hasSuffix("first/results"))
    #expect(saved.sources[3].hasSuffix("group/results/chain-02"))
    #expect(saved.seeds == [42, 43, 50, 51])
    #expect(saved.file.path.hasPrefix(first.directory.appendingPathComponent("comparisons").path))
    #expect(!saved.report.statistics.isEmpty)

    let reopened = RunComparisonModel(entry: first, executable: engine)
    await reopened.load()
    #expect(
      reopened.comparisons.map(\.id) == [saved.id],
      Comment(rawValue: "\(reopened.comparisons.map(\.id)) vs \(saved.id)"))

    model.selected = [second.id]
    model.burnin = "150"
    #expect(!model.valid)
    model.burnin = "10"
    let process = RunComparisonProcess()
    process.cancel()
    await #expect(throws: CancellationError.self) {
      try await process.execute(
        directories: RunComparisonModel.chainDirectories(first)
          + RunComparisonModel.chainDirectories(second),
        executable: engine, burnin: 10,
        output: folder.appendingPathComponent("cancelled.json"))
    }
    #expect(!FileManager.default.fileExists(atPath: folder.appendingPathComponent("cancelled.json").path))
  }
}
