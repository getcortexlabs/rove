import Foundation

/// A native history wrapper around a completed CLI run. The original results
/// remain in place; the wrapper supplies the setup/session files used by Swift.
enum CommandLineImport {
  static func prepare(_ source: URL, matrix: URL, under parent: URL, executable: URL) async throws
    -> URL
  {
    let record = try EngineRecord.read(source.appendingPathComponent("run.json"))
    guard record.status == "completed", record.kind == nil || record.kind == "independent_chains"
    else {
      throw ImportError.message("Choose a completed Rove sampling run.")
    }
    let settings = try settings(from: record)
    let report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: settings.gapsAsMissing, partitioning: settings.partitioning,
      executable: executable)
    guard report.inputSha256 == record.input.sha256 else {
      throw ImportError.message("The selected matrix does not match this run.")
    }
    let draft = AnalysisDraft(
      matrix: matrix, report: report, settings: settings, monophyly: record.model.monophyly)
    try draft.validate()
    let directory = parent.appendingPathComponent("imported-\(UUID())")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    do {
      try FileManager.default.createSymbolicLink(
        at: directory.appendingPathComponent("results"), withDestinationURL: source)
      _ = try EngineRecord.verify(directory: directory, draft: draft)
      try draft.write(to: directory.appendingPathComponent("setup.rove.json"))
      let session = SessionRecord(
        formatVersion: 1, id: UUID(), status: "completed", setup: draft,
        exitCode: 0, error: nil,
        startedAt: record.startedUnixSeconds.map(Date.init(timeIntervalSince1970:)),
        finishedAt: nil, elapsedSeconds: record.wallSeconds, completedSteps: record.completedSteps)
      let encoder = JSONEncoder()
      encoder.dateEncodingStrategy = .iso8601
      encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
      try encoder.encode(session).write(
        to: directory.appendingPathComponent("session.json"), options: .atomic)
      return directory
    } catch {
      try? FileManager.default.removeItem(at: directory)
      throw error
    }
  }

  static func settings(from record: EngineRecord) throws -> AnalysisSettings {
    func option(_ flag: String) -> String? {
      guard let index = record.command.firstIndex(of: flag),
        record.command.indices.contains(index + 1)
      else { return nil }
      return record.command[index + 1]
    }
    func overrides(_ flag: String) throws -> [MoveOverride] {
      try record.command.indices.filter { record.command[$0] == flag }.map { index in
        guard record.command.indices.contains(index + 1) else { throw RunFailure.invalidCompletion }
        let parts = record.command[index + 1].split(separator: "=", maxSplits: 1)
        guard parts.count == 2, let value = Double(parts[1]) else {
          throw RunFailure.invalidCompletion
        }
        return MoveOverride(move: String(parts[0]), value: value)
      }
    }
    guard let steps = Int(exactly: record.settings.steps),
      let interval = Int(exactly: record.settings.sampleEvery),
      let conditioning = record.model.conditioningKind
        ?? option("--conditioning")
    else { throw RunFailure.invalidCompletion }
    var value = AnalysisSettings()
    value.dataType = record.model.resolvedDataType
    guard record.model.dataModelMatches(value) else { throw RunFailure.invalidCompletion }
    value.treePrior = record.model.treePriorKind ?? "birth-death"
    if value.treePrior == "yule" {
      guard let prior = record.model.birthRatePriorParameters, prior.valid else {
        throw RunFailure.invalidCompletion
      }
      value.birthRateUpper = String(prior.upper)
    }
    guard record.model.treePriorMatches(value) else { throw RunFailure.invalidCompletion }
    value.clock = record.model.preset
    let sigma = record.model.clockSigmaPriorParameters ?? .reference
    value.clockSigmaShape = String(sigma.shape)
    value.clockSigmaScale = String(sigma.scale)
    value.conditioning = conditioning
    value.partitioning = record.model.partitioning ?? "declared"
    value.steps = steps
    value.sampleEvery = interval
    value.seed = String(record.settings.seed)
    value.gapsAsMissing = record.input.gaps == "missing"
    value.chains = record.chainCount ?? 1
    value.backend = record.settings.backend ?? "automatic"
    value.comparisonBurnin = option("--burnin-percent").flatMap(Int.init) ?? 10
    value.printEvery = option("--print-every").flatMap(Int.init)
    value.stopWhenConverged = record.command.contains("--stop-when-converged")
    value.sampleFromPrior = record.model.sampleFromPrior ?? false
    value.minESS = option("--min-ess").flatMap(Double.init)
    value.maxRhat = option("--max-rhat").flatMap(Double.init)
    value.checkEvery = option("--check-every").flatMap(Int.init)
    value.checkpointEvery = option("--checkpoint-every").flatMap(Int.init)
    value.tuning = record.settings.tuning ?? true
    value.weights = try overrides("--weight")
    value.tuningValues = try overrides("--tuning")
    if let model = record.model.dating { value.dating = DatingSettings(restoring: model) }
    if let error = value.validationError { throw ImportError.message(error) }
    return value
  }
}

extension DatingSettings {
  init(restoring model: EngineDatingModel) {
    self.init()
    enabled = true
    timeUnit = model.timeUnit
    birthRate = String(model.birthRate)
    deathRate = String(model.deathRate)
    fossilSamplingRate = String(model.fossilSamplingRate)
    presentSampling = String(model.presentSampling)
    origin = String(model.origin)
    clockRate = String(model.clockRate)
    clock = model.clockPriorMean == nil ? .fixed : .estimated
    clockPriorMean = model.clockPriorMean.map(String.init(describing:)) ?? ""
    ages = model.ages.mapValues {
      switch $0 {
      case .fixed(0): TipAgeInput()
      case .fixed(let age): TipAgeInput(kind: .fixed, fixed: String(age))
      case .interval(let lower, let upper):
        TipAgeInput(kind: .interval, minimum: String(lower), maximum: String(upper))
      }
    }
    if let priors = model.processPriors {
      func rate(_ mean: Double?) -> ProcessRateInput {
        ProcessRateInput(estimated: mean != nil, mean: mean.map(String.init(describing:)) ?? "")
      }
      var estimation = ProcessEstimation(
        birth: rate(priors.birthRateMean), death: rate(priors.deathRateMean),
        fossilSampling: rate(priors.fossilSamplingRateMean), origin: rate(priors.originExcessMean))
      if let beta = priors.presentSamplingBeta, beta.count == 2 {
        estimation.estimateSampling = true
        estimation.samplingAlpha = String(beta[0])
        estimation.samplingBeta = String(beta[1])
      }
      processEstimation = estimation
    }
  }
}
