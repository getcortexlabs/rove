import Foundation
import Testing

@testable import RoveApp

struct RunAssessmentTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL { root.appendingPathComponent("target/release/rove") }

  @Test @MainActor func reviewPersistsAndExportReopensWithVerifiedFiles() async throws {
    let parent = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-review-\(UUID())")
    try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: parent) }
    let matrix = parent.appendingPathComponent("matrix.nex")
    try
      "#NEXUS BEGIN DATA; DIMENSIONS NTAX=4 NCHAR=4; FORMAT SYMBOLS=\"01\"; MATRIX A 0000 B 0011 C 1100 D 1111; END;"
      .write(to: matrix, atomically: true, encoding: .utf8)
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    for chains in [1, 2] {
      var settings = AnalysisSettings()
      settings.steps = 100
      settings.sampleEvery = 10
      settings.chains = chains
      let draft = AnalysisDraft(matrix: matrix, report: report, settings: settings)
      let directory = parent.appendingPathComponent("run-\(chains)")
      let result = await RunProcess().execute(
        RunRequest(
          id: UUID(), executable: engine,
          directory: directory, draft: draft)
      ) { _ in }
      #expect(result.status == .completed)
      let entry = HistoryEntry.read(directory)
      let model = RunAssessmentModel(entry: entry, executable: engine)
      model.options.burnin = "20"
      model.start()
      await model.waitForExit()
      #expect(model.error == nil, Comment(rawValue: model.details ?? ""))
      let review = try #require(model.report)
      #expect(review.burninPercent == 20 && !review.convergenceCertified)
      #expect(review.sections.first(where: { $0.id == "completion" })?.status == "passed")
      #expect(
        review.sections.first(where: { $0.id == "agreement" })?.status
          == (chains == 1 ? "unavailable" : "attention"))
      let restored = RunAssessmentModel(entry: entry, executable: engine)
      await restored.load()
      #expect(restored.options.burnin == "20" && restored.report?.chainCount == chains)
      let package = parent.appendingPathComponent("package-\(chains)")
      model.options.includeMatrix = chains == 2
      model.start(exportTo: package)
      await model.waitForExit()
      #expect(model.error == nil, Comment(rawValue: model.details ?? ""))
      #expect(model.exported == package)
      #expect(
        FileManager.default.fileExists(atPath: package.appendingPathComponent("matrix.nex").path)
          == (chains == 2))
      #expect(HistoryEntry.read(package.appendingPathComponent("run")).status == .completed)
      let manifest = try #require(
        JSONSerialization.jsonObject(
          with: Data(contentsOf: package.appendingPathComponent("manifest.json"))) as? [String: Any]
      )
      let files = try #require(manifest["files"] as? [[String: Any]])
      for file in files {
        let path = try #require(file["path"] as? String)
        let hash = try #require(file["sha256"] as? String)
        #expect(try TreeSource.fingerprint(package.appendingPathComponent(path)) == hash)
      }
      #expect(HistoryEntry.read(directory).status == .completed)
    }
    let entry = HistoryEntry.read(parent.appendingPathComponent("run-1"))
    try Data("changed".utf8).write(to: matrix)
    let changed = RunAssessmentModel(entry: entry, executable: engine)
    changed.start()
    await changed.waitForExit()
    #expect(changed.report?.originalMatrixStatus == "changed")
    changed.options.includeMatrix = true
    let rejected = parent.appendingPathComponent("rejected")
    changed.start(exportTo: rejected)
    await changed.waitForExit()
    #expect(changed.error != nil && !FileManager.default.fileExists(atPath: rejected.path))
  }

  @Test func histogramIncludesEveryPairAndHandlesConstantDifferences() {
    let constant = PredictiveDistribution(name: "example", differences: [0, 0, 0], undefined: 2)
    #expect(constant.bins.count == 1 && constant.bins[0].count == 3)
    let spread = PredictiveDistribution(
      name: "example", differences: [-2, -1, 0, 1, 4], undefined: 0)
    #expect(spread.bins.map(\.count).reduce(0, +) == 5)
    #expect(PredictiveDistribution(name: "undefined", differences: [], undefined: 2).bins.isEmpty)
  }
}
