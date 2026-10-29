import Foundation

struct RunEvent: Decodable, Sendable {
  let schemaVersion: Int
  let event: String
  let step: UInt64?
  let totalSteps: UInt64?
  let elapsedSeconds: Double?
  let inputSha256: String?
  let posterior: Double?
  let likelihood: Double?
  let prior: Double?
  let message: String?
  var chain: Int? = nil
  var chainCount: Int? = nil
  var terminationReason: String? = nil
}

struct RunEventReader {
  let steps: UInt64
  let fingerprint: String
  private(set) var terminal: RunEvent?
  let chains: Int
  let resumedStep: UInt64?
  private struct ChainState {
    var started = false
    var step: UInt64 = 0
    var seconds: Double = 0
    var terminal: RunEvent?
  }
  private var states: [ChainState]
  private var pending = Data()

  init(steps: UInt64, fingerprint: String, chains: Int = 1, resumedStep: UInt64? = nil) {
    self.steps = steps
    self.fingerprint = fingerprint
    self.chains = chains
    self.resumedStep = resumedStep
    states = Array(repeating: ChainState(), count: chains)
  }

  mutating func append(_ data: Data) throws -> [RunEvent] {
    pending.append(data)
    var events: [RunEvent] = []
    while let newline = pending.firstIndex(of: 10) {
      let line = Data(pending[..<newline])
      pending.removeSubrange(...newline)
      guard !line.isEmpty, line.count <= 65_536 else { throw RunFailure.protocolError }
      let decoder = JSONDecoder()
      decoder.keyDecodingStrategy = .convertFromSnakeCase
      let event: RunEvent
      do { event = try decoder.decode(RunEvent.self, from: line) } catch {
        throw RunFailure.protocolError
      }
      try validate(event)
      events.append(event)
    }
    guard pending.count <= 65_536 else { throw RunFailure.protocolError }
    return events
  }

  func finish() throws {
    guard pending.isEmpty, terminal != nil else { throw RunFailure.protocolError }
  }

  private mutating func validate(_ value: RunEvent) throws {
    guard value.schemaVersion == 1, terminal == nil else { throw RunFailure.protocolError }
    if value.event == "failed" {
      guard let message = value.message, !message.isEmpty else { throw RunFailure.protocolError }
      terminal = value
      return
    }
    if value.event.hasPrefix("group_") {
      guard chains > 1, value.chain == nil, value.chainCount == chains,
        value.inputSha256 == fingerprint, value.totalSteps == steps,
        value.step == states.map(\.step).min(),
        let seconds = value.elapsedSeconds, seconds.isFinite, seconds >= 0,
        states.allSatisfy({ $0.terminal != nil }),
        ["group_completed", "group_cancelled", "group_paused"].contains(value.event)
      else { throw RunFailure.protocolError }
      if value.event == "group_completed" {
        guard states.allSatisfy({ $0.terminal?.event == "completed" }),
          states.allSatisfy({ $0.terminal?.terminationReason == value.terminationReason }),
          states.allSatisfy({ $0.step == value.step })
        else { throw RunFailure.protocolError }
      }
      if value.event == "group_paused" {
        guard states.allSatisfy({ $0.terminal?.event == "paused" && $0.step == value.step }) else {
          throw RunFailure.protocolError
        }
      }
      terminal = value
      return
    }
    let number = value.chain ?? 1
    guard number >= 1, number <= chains,
      chains == 1 ? value.chain == nil : value.chain != nil
    else { throw RunFailure.protocolError }
    let index = number - 1
    guard states[index].terminal == nil else { throw RunFailure.protocolError }
    var state = states[index]
    guard let step = value.step, let seconds = value.elapsedSeconds,
      value.totalSteps == steps, step <= steps, seconds.isFinite, seconds >= state.seconds,
      value.inputSha256 == fingerprint,
      [value.posterior, value.likelihood, value.prior].allSatisfy({ $0?.isFinite == true })
    else { throw RunFailure.protocolError }
    switch value.event {
    case "started":
      guard !state.started, step == (resumedStep ?? 0) else { throw RunFailure.protocolError }
      state.started = true
    case "progress":
      guard state.started, step > state.step else { throw RunFailure.protocolError }
    case "completed", "cancelled", "paused":
      guard state.started, step >= state.step,
        value.event != "completed" || step == steps
          || (chains > 1 && value.terminationReason == "diagnostic_thresholds"),
        !["cancelled", "paused"].contains(value.event) || step < steps
      else { throw RunFailure.protocolError }
      state.terminal = value
      if chains == 1 { terminal = value }
    default: throw RunFailure.protocolError
    }
    state.step = step
    state.seconds = seconds
    states[index] = state
  }
}

enum RunFailure: LocalizedError {
  case invalidCheckpoint
  case runInUse
  case protocolError
  case changedInput
  case invalidCompletion
  var errorDescription: String? {
    switch self {
    case .invalidCheckpoint: "The checkpoint does not match this analysis. Inspect the run folder."
    case .runInUse: "This analysis is already open in another process."
    case .protocolError: "The engine returned invalid or incomplete progress events."
    case .changedInput:
      "The matrix changed after it was imported. Import it again and review the setup."
    case .invalidCompletion: "The engine stopped without a verified set of completed outputs."
    }
  }
}
