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
}

struct RunEventReader {
  let steps: UInt64
  let fingerprint: String
  private(set) var terminal: RunEvent?
  private var started = false
  private var previousStep: UInt64 = 0
  private var previousTime: Double = 0
  private var pending = Data()

  init(steps: UInt64, fingerprint: String) {
    self.steps = steps
    self.fingerprint = fingerprint
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
    guard let step = value.step, let seconds = value.elapsedSeconds,
      value.totalSteps == steps, step <= steps, seconds.isFinite, seconds >= previousTime,
      value.inputSha256 == fingerprint,
      [value.posterior, value.likelihood, value.prior].allSatisfy({ $0?.isFinite == true })
    else { throw RunFailure.protocolError }
    switch value.event {
    case "started":
      guard !started, step == 0 else { throw RunFailure.protocolError }
      started = true
    case "progress":
      guard started, step > previousStep else { throw RunFailure.protocolError }
    case "completed", "cancelled":
      guard started, step >= previousStep,
        value.event != "completed" || step == steps,
        value.event != "cancelled" || step < steps
      else { throw RunFailure.protocolError }
      terminal = value
    default: throw RunFailure.protocolError
    }
    previousStep = step
    previousTime = seconds
  }
}

enum RunFailure: LocalizedError {
  case protocolError
  case changedInput
  case invalidCompletion
  var errorDescription: String? {
    switch self {
    case .protocolError: "The engine returned invalid or incomplete progress events."
    case .changedInput:
      "The matrix changed after it was imported. Import it again and review the setup."
    case .invalidCompletion: "The engine stopped without a verified set of completed outputs."
    }
  }
}
