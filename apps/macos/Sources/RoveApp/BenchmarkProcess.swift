import Foundation

struct BenchmarkEvent: Decodable, Sendable {
  let event: String
  let `protocol`: BenchmarkProtocol?
  let protocolSHA256: String?
  let engine: BenchmarkEngine?
  let sample: BenchmarkSample?
  let chains: Int?
  let repetition: Int?
}

final class BenchmarkProcess: @unchecked Sendable {
  private let lock = NSLock()
  private var input: FileHandle?
  private var cancelled = false

  func cancel() {
    lock.lock()
    defer { lock.unlock() }
    cancelled = true
    try? input?.close()
    input = nil
  }

  func execute(executable: URL, onEvent: @escaping @Sendable (BenchmarkEvent) -> Void) async throws
    -> [BenchmarkEvent]
  {
    try await withCheckedThrowingContinuation {
      (continuation: CheckedContinuation<[BenchmarkEvent], Error>) in
      Thread.detachNewThread {
        continuation.resume(with: Result { try self.run(executable: executable, onEvent: onEvent) })
      }
    }
  }

  private func run(executable: URL, onEvent: @escaping @Sendable (BenchmarkEvent) -> Void) throws
    -> [BenchmarkEvent]
  {
    let hash = try benchmarkFileHash(executable)
    let pipe = Pipe()
    let owner = Pipe()
    let process = Process()
    process.executableURL = executable
    process.arguments = ["benchmark", "--cancel-on-stdin-close"]
    process.standardOutput = pipe
    process.standardInput = owner
    process.standardError = FileHandle.nullDevice
    defer {
      try? owner.fileHandleForWriting.close()
      try? owner.fileHandleForReading.close()
      try? pipe.fileHandleForReading.close()
      try? pipe.fileHandleForWriting.close()
    }
    lock.lock()
    if cancelled {
      lock.unlock()
      throw BenchmarkError.cancelled
    }
    do { try process.run() } catch {
      lock.unlock()
      throw error
    }
    input = owner.fileHandleForWriting
    lock.unlock()
    try? owner.fileHandleForReading.close()
    try? pipe.fileHandleForWriting.close()
    defer {
      if process.isRunning { cancel() }
      process.waitUntilExit()
      lock.lock()
      input = nil
      lock.unlock()
    }
    var events: [BenchmarkEvent] = []
    var pending = Data()
    var started = false
    var terminal = false
    var completed = false
    var samples = 0
    var warming = false
    while true {
      let bytes = pipe.fileHandleForReading.availableData
      if bytes.isEmpty { break }
      pending.append(bytes)
      guard pending.count <= 65536 else { throw BenchmarkError.invalid }
      while let end = pending.firstIndex(of: 10) {
        let line = pending.prefix(upTo: end)
        pending.removeSubrange(...end)
        let event = try JSONDecoder().decode(BenchmarkEvent.self, from: line)
        guard !terminal else { throw BenchmarkError.invalid }
        switch event.event {
        case "started":
          guard !started, event.engine?.sha256 == hash,
            event.protocolSHA256 == (try benchmarkHash(BenchmarkProtocol.data)),
            event.protocol
              == (try JSONDecoder().decode(BenchmarkProtocol.self, from: BenchmarkProtocol.data))
          else { throw BenchmarkError.invalid }
          started = true
        case "warming":
          guard started, !warming, samples < 6, event.chains == (samples < 3 ? 1 : 4),
            event.repetition == samples % 3
          else { throw BenchmarkError.invalid }
          warming = true
        case "measurement":
          guard started, warming, let sample = event.sample,
            sample.chains == (samples < 3 ? 1 : 4), sample.repetition == samples % 3,
            sample.seconds.isFinite, sample.seconds > 0
          else { throw BenchmarkError.invalid }
          warming = false
          samples += 1
        case "completed":
          guard started, samples == 6, !warming else { throw BenchmarkError.invalid }
          terminal = true
          completed = true
        case "cancelled": terminal = true
        default: throw BenchmarkError.invalid
        }
        events.append(event)
        onEvent(event)
      }
    }
    process.waitUntilExit()
    lock.lock()
    let stopped = cancelled
    lock.unlock()
    if stopped || process.terminationStatus == 130 { throw BenchmarkError.cancelled }
    guard pending.isEmpty, terminal, completed, process.terminationStatus == 0 else {
      throw BenchmarkError.failed
    }
    return events
  }
}
