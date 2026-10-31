import Foundation
import Testing

@testable import RoveApp

struct ModelToolsTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }

  private func folder(_ name: String) throws -> URL {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-\(name)-\(UUID())")
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false)
    return folder
  }

  private func copy(_ fixture: String, to folder: URL, as name: String) throws -> URL {
    let target = folder.appendingPathComponent(name)
    try Data(contentsOf: root.appendingPathComponent(fixture)).write(to: target)
    return target
  }

  private func run(
    _ directory: URL, matrix: URL, seed: String, clock: String = "reference", chains: Int = 1,
    steps: Int = 2_000, prior: Bool = false
  ) async throws -> HistoryEntry {
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    var settings = AnalysisSettings()
    settings.steps = steps
    settings.sampleEvery = 100
    settings.seed = seed
    settings.clock = clock
    settings.chains = chains
    settings.sampleFromPrior = prior
    let request = RunRequest(
      id: UUID(), executable: engine, directory: directory,
      draft: AnalysisDraft(matrix: matrix, report: report, settings: settings))
    #expect(await RunProcess().execute(request) { _ in }.status == .completed)
    return HistoryEntry.read(directory)
  }

  @Test @MainActor func predictiveChecksRunOnOneChainAndAreSaved() async throws {
    let folder = try folder("predict")
    defer { try? FileManager.default.removeItem(at: folder) }
    let matrix = try copy("fixtures/reference/20x50/matrix.nex", to: folder, as: "matrix.nex")
    let single = try await run(folder.appendingPathComponent("single"), matrix: matrix, seed: "42")
    let group = try await run(
      folder.appendingPathComponent("group"), matrix: matrix, seed: "50", chains: 2, steps: 1_000)
    let prior = try await run(
      folder.appendingPathComponent("prior"), matrix: matrix, seed: "7", steps: 500, prior: true)

    let model = PredictiveCheckModel(entry: single, executable: engine)
    #expect(model.available && model.chains == 1 && model.valid && model.seed == "42")
    model.replicates = "0"
    #expect(!model.valid)
    model.replicates = "5"
    model.seed = "3"
    model.start()
    await model.waitForExit()
    #expect(model.error == nil, Comment(rawValue: model.details ?? ""))
    #expect(model.completed == 5)
    let saved = try #require(model.predictions.first)
    // 2,000 steps sampled every 100 give 21 samples; 10% burn-in discards 2.
    let source = saved.report.source
    #expect(source.totalSamples == 21 && source.discardedSamples == 2 && source.retainedSamples == 19)
    #expect(saved.report.replicates == 5 && saved.report.seed == 3)
    #expect(saved.report.global["parsimony_length"]?.validPairs == 5)
    #expect(
      PredictiveReport.statistics.allSatisfy { name in
        saved.report.global[name].map { $0.validPairs + $0.undefinedPairs == 5 } == true
      })
    for file in ["predictive.json", "replicates.jsonl", "characters.tsv", "matrices/replicate-00001.nex"] {
      #expect(FileManager.default.fileExists(atPath: saved.directory.appendingPathComponent(file).path))
    }
    #expect(saved.directory.deletingLastPathComponent().lastPathComponent == "predictions")
    await model.load()
    #expect(model.predictions.count == 1 && model.predictions.first?.id == saved.id)

    // A group is checked one chain at a time.
    let grouped = PredictiveCheckModel(entry: group, executable: engine)
    #expect(grouped.chains == 2)
    grouped.chain = 2
    grouped.replicates = "3"
    #expect(grouped.source.lastPathComponent == "chain-02")
    grouped.start()
    await grouped.waitForExit()
    #expect(grouped.error == nil, Comment(rawValue: grouped.details ?? ""))
    #expect(grouped.predictions.first?.report.source.directory.hasSuffix("chain-02") == true)
    #expect(grouped.predictions.first?.report.source.totalSamples == 11)

    #expect(!PredictiveCheckModel(entry: prior, executable: engine).available)

    // The engine's own refusal reaches the details; a missing matrix is caught first.
    let other = try copy("fixtures/synthetic/control_20x50.nex", to: folder, as: "other.nex")
    var request = PredictiveRequest(
      source: model.source, matrix: other,
      output: folder.appendingPathComponent("predictions/rejected"), executable: engine,
      burnin: 10, replicates: 2, seed: 1)
    await #expect(throws: ImportError.self) { try await request.execute(with: EngineTool()) { _ in } }
    do {
      _ = try await request.execute(with: EngineTool()) { _ in }
    } catch {
      #expect(error.localizedDescription.contains("fingerprint"))
    }
    #expect(!FileManager.default.fileExists(atPath: request.output.path))
    request = PredictiveRequest(
      source: model.source, matrix: folder.appendingPathComponent("gone.nex"),
      output: request.output, executable: engine, burnin: 10, replicates: 2, seed: 1)
    do {
      _ = try await request.execute(with: EngineTool()) { _ in }
      Issue.record("a missing matrix was accepted")
    } catch {
      #expect(error.localizedDescription.contains("recorded path"))
    }
  }

  @Test func engineToolKeepsTheLastLinesAndTheExitStatus() async throws {
    let tool = EngineTool()
    let lines = Lines()
    let outcome = try await tool.execute(
      executable: URL(fileURLWithPath: "/bin/sh"),
      arguments: ["-c", "printf 'first\\nsecond' >&2; exit 3"]
    ) { lines.append($0) }
    #expect(outcome.status == 3 && outcome.log == "first\nsecond")
    #expect(lines.values == ["first", "second"])
    // A finished command is not reported as cancelled, whatever the flag says.
    let late = EngineTool()
    let done = try await late.execute(
      executable: URL(fileURLWithPath: "/bin/sh"), arguments: ["-c", "exit 0"]) { _ in }
    late.cancel()
    #expect(done.status == 0)
    let early = EngineTool()
    early.cancel()
    await #expect(throws: CancellationError.self) {
      try await early.execute(executable: URL(fileURLWithPath: "/bin/sh"), arguments: ["-c", "exit 0"]) { _ in }
    }
  }

  @Test @MainActor func marginalEstimatesSaveAndCompareAcrossRuns() async throws {
    let folder = try folder("marginal")
    defer { try? FileManager.default.removeItem(at: folder) }
    let matrix = try copy("fixtures/reference/20x50/matrix.nex", to: folder, as: "matrix.nex")
    let other = try copy("fixtures/synthetic/control_20x50.nex", to: folder, as: "control.nex")
    let relaxed = try await run(folder.appendingPathComponent("relaxed"), matrix: matrix, seed: "42")
    let strict = try await run(
      folder.appendingPathComponent("strict"), matrix: matrix, seed: "43", clock: "strict")
    let control = try await run(folder.appendingPathComponent("control"), matrix: other, seed: "9")

    let model = MarginalLikelihoodModel(entry: relaxed, executable: engine)
    #expect(model.available && model.sampleEvery == 100 && model.steps == "200" && model.seed == "42")
    model.powers = "4"
    model.alpha = "0.3"
    model.steps = "400"
    model.burnin = "25"
    model.seed = "1"
    let schedule = try #require(model.schedule)
    let arguments = try schedule.arguments(
      draft: try #require(relaxed.draft), output: URL(fileURLWithPath: "/tmp/estimate"))
    #expect(arguments.first == "marginal")
    #expect(contiguous(["--powers", "4", "--alpha", "0.3", "--burnin-percent", "25"], in: arguments))
    #expect(contiguous(["--model", "reference", "--conditioning", "variable"], in: arguments))
    #expect(!arguments.contains("--chains") && arguments.contains("--expect-input-sha256"))
    model.steps = "450"
    #expect(!model.valid)
    model.steps = "400"
    model.start()
    await model.waitForExit()
    #expect(model.error == nil, Comment(rawValue: model.details ?? ""))
    let own = try #require(model.estimates.first)
    #expect(own.estimate.powers.count == 4 && own.estimate.settings.stepsPerPower == 400)
    #expect(own.estimate.settings.burninPercent == 25 && own.estimate.settings.seed == 1)
    // Each power keeps 5 samples including the state before the first step; 25% discards 1.
    #expect(own.estimate.powers.allSatisfy { $0.samples == 4 })
    #expect(own.estimate.steppingStone.logMarginalLikelihood.isFinite)
    #expect(own.record.model.conditioningLabel == "Mkv" && own.record.model.preset == "reference")
    #expect(own.directory.deletingLastPathComponent().lastPathComponent == "marginal")

    let strictModel = MarginalLikelihoodModel(entry: strict, executable: engine)
    strictModel.powers = "4"
    strictModel.steps = "400"
    strictModel.seed = "2"
    strictModel.start()
    await strictModel.waitForExit()
    #expect(strictModel.error == nil, Comment(rawValue: strictModel.details ?? ""))
    #expect(MarginalLikelihoodModel.comparable(relaxed, strict))
    #expect(!MarginalLikelihoodModel.comparable(relaxed, control))
    #expect(!MarginalLikelihoodModel.comparable(relaxed, relaxed))
    await model.load(history: [relaxed, strict, control])
    #expect(model.estimates.count == 1 && model.others.count == 1)
    let theirs = try #require(model.others.first)
    #expect(theirs.run.lastPathComponent == "strict" && theirs.record.model.preset == "strict")
    let delta = MarginalLikelihoodModel.difference(
      own.estimate.steppingStone, theirs.estimate.steppingStone)
    #expect(
      delta.value
        == own.estimate.steppingStone.logMarginalLikelihood
        - theirs.estimate.steppingStone.logMarginalLikelihood)
    if let a = own.estimate.steppingStone.standardError,
      let b = theirs.estimate.steppingStone.standardError
    {
      #expect(delta.standardError == (a * a + b * b).squareRoot())
    } else {
      #expect(delta.standardError == nil)
    }

    #expect(MarginalLikelihoodModel.progress(from: "0.300000\t20\t-1.0\t-2.0\t3.0\t0.1")?.step == 20)
    #expect(MarginalLikelihoodModel.progress(from: "0.300000\t20\t-1.0\t-2.0\t3.0\t0.1")?.power == "0.300000")
    #expect(MarginalLikelihoodModel.progress(from: "power\tstep\tposterior\tlikelihood\tprior\tseconds") == nil)

    // A cancelled estimate is not listed; its folder keeps the partial traces.
    let long = MarginalLikelihoodModel(entry: relaxed, executable: engine)
    long.powers = "4"
    long.steps = "5000000"
    long.seed = "5"
    long.start()
    let estimates = relaxed.directory.appendingPathComponent("marginal")
    func partial() -> URL? {
      let children =
        (try? FileManager.default.contentsOfDirectory(at: estimates, includingPropertiesForKeys: nil))
        ?? []
      return children.first {
        $0.lastPathComponent != own.directory.lastPathComponent
          && FileManager.default.fileExists(
            atPath: $0.appendingPathComponent("power-03/trace.log").path)
      }
    }
    for _ in 0..<100 where partial() == nil { try await Task.sleep(for: .milliseconds(50)) }
    let cancelled = try #require(partial())
    long.cancel()
    await long.waitForExit()
    #expect(long.error?.contains("cancelled") == true && long.estimates.isEmpty)
    let record = try MarginalRunRecord.read(cancelled.appendingPathComponent("run.json"))
    #expect(record.status == "cancelled" && record.kind == "marginal_likelihood_run")
    #expect(throws: (any Error).self) { try SavedEstimate.read(cancelled, run: relaxed.directory) }
    await model.load(history: [relaxed, strict])
    #expect(model.estimates.count == 1)

    // An estimate of another model under this run's folder is not this run's.
    var changed = try #require(relaxed.draft?.settings)
    changed.conditioning = "none"
    #expect(!own.matches(AnalysisDraft(matrix: matrix, report: try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine), settings: changed)))
    changed = try #require(relaxed.draft?.settings)
    changed.sampleEvery = 50
    #expect(!own.matches(AnalysisDraft(matrix: matrix, report: try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine), settings: changed)))
  }

  private func contiguous(_ part: [String], in whole: [String]) -> Bool {
    whole.indices.contains { whole[$0...].starts(with: part) }
  }

  private final class Lines: @unchecked Sendable {
    private let lock = NSLock()
    private var stored: [String] = []
    var values: [String] {
      lock.lock()
      defer { lock.unlock() }
      return stored
    }
    func append(_ line: String) {
      lock.lock()
      stored.append(line)
      lock.unlock()
    }
  }
}
