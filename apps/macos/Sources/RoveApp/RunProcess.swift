import CryptoKit
import Darwin
import Foundation

struct RunRequest: Sendable {
  let id: UUID
  let executable: URL
  let directory: URL
  let draft: AnalysisDraft
  var output: URL { directory.appendingPathComponent("results") }
  var arguments: [String] {
    draft.settings.arguments(
      matrix: URL(fileURLWithPath: draft.matrixPath), output: output,
      monophyly: draft.monophyly) + [
        "--events-json", "--cancel-on-stdin-close", "--expect-input-sha256", draft.matrixSHA256,
        "--print-every", String(max(1, min(100_000, draft.settings.steps / 100))),
      ]
  }
}

enum RunStatus: String, Codable, Sendable {
  case preparing, running, cancelling, completed, cancelled, failed
  var isActive: Bool { self == .preparing || self == .running || self == .cancelling }
}

struct RunResult: Sendable {
  let status: RunStatus
  let exitCode: Int32?
  let error: String?
  let details: String?
}

private struct SessionRecord: Encodable {
  let formatVersion = 1
  let id: UUID
  let status: String
  let setup: AnalysisDraft
  let exitCode: Int32?
  let error: String?
}

private struct EngineRecord: Decodable {
  struct Input: Decodable { let sha256: String }
  struct Settings: Decodable {
    let steps: UInt64
    let sampleEvery: UInt64
    let seed: UInt64
  }
  struct Model: Decodable { let monophyly: [[String]] }
  let status: String
  let completedSteps: UInt64
  let input: Input
  let settings: Settings
  let model: Model
}

// The worker owns all blocking I/O. The lock only protects process ownership
// and cancellation shared with the UI; it is never held while waiting for exit.
final class RunProcess: @unchecked Sendable {
  private let lock = NSLock()
  private var process: Process?
  private var input: FileHandle?
  private var requestedCancellation = false

  func cancel() {
    lock.lock()
    requestedCancellation = true
    try? input?.close()
    input = nil
    let child = process
    lock.unlock()
    guard let child else { return }
    DispatchQueue.global().asyncAfter(deadline: .now() + 2) { [weak self] in
      self?.terminateIfOwned(child, force: false)
      DispatchQueue.global().asyncAfter(deadline: .now() + 2) { [weak self] in
        self?.terminateIfOwned(child, force: true)
      }
    }
  }

  private func terminateIfOwned(_ child: Process, force: Bool) {
    lock.lock()
    defer { lock.unlock() }
    guard process === child, child.isRunning else { return }
    if force { _ = Darwin.kill(child.processIdentifier, SIGKILL) } else { child.terminate() }
  }

  private var cancellationRequested: Bool {
    lock.lock()
    defer { lock.unlock() }
    return requestedCancellation
  }

  func execute(_ request: RunRequest, onEvent: @Sendable (RunEvent) -> Void) -> RunResult {
    var result: RunResult
    var launched: Process?
    var ownsDirectory = false
    do {
      if cancellationRequested {
        return RunResult(status: .cancelled, exitCode: nil, error: nil, details: nil)
      }
      if let issue = request.draft.settings.validationError { throw ImportError.message(issue) }
      let matrix = URL(fileURLWithPath: request.draft.matrixPath)
      let values = try matrix.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
      guard values.isRegularFile == true, (values.fileSize ?? 0) <= 32 * 1024 * 1024 else {
        throw RunFailure.changedInput
      }
      let fingerprint = SHA256.hash(data: try Data(contentsOf: matrix)).map {
        String(format: "%02x", $0)
      }.joined()
      guard fingerprint == request.draft.matrixSHA256 else { throw RunFailure.changedInput }
      try FileManager.default.createDirectory(
        at: request.directory.deletingLastPathComponent(), withIntermediateDirectories: true)
      try FileManager.default.createDirectory(
        at: request.directory, withIntermediateDirectories: false)
      ownsDirectory = true
      try request.draft.write(to: request.directory.appendingPathComponent("setup.rove.json"))
      try writeRecord(request, status: "partial", result: nil)
      let errors = request.directory.appendingPathComponent("engine.stderr.log")
      let events = request.directory.appendingPathComponent("events.jsonl")
      FileManager.default.createFile(atPath: errors.path, contents: nil)
      FileManager.default.createFile(atPath: events.path, contents: nil)
      let stderr = try FileHandle(forWritingTo: errors)
      let eventLog = try FileHandle(forWritingTo: events)
      defer {
        try? stderr.close()
        try? eventLog.close()
      }
      let pipe = Pipe()
      let owner = Pipe()
      defer {
        try? pipe.fileHandleForReading.close()
        try? owner.fileHandleForReading.close()
        try? owner.fileHandleForWriting.close()
      }
      let child = Process()
      child.executableURL = request.executable
      child.arguments = request.arguments
      child.standardOutput = pipe
      child.standardError = stderr
      child.standardInput = owner
      lock.lock()
      do { try child.run() } catch {
        lock.unlock()
        throw error
      }
      process = child
      input = owner.fileHandleForWriting
      let cancelledBeforeLaunch = requestedCancellation
      lock.unlock()
      launched = child
      try? owner.fileHandleForReading.close()
      try? pipe.fileHandleForWriting.close()
      if cancelledBeforeLaunch { cancel() }
      var reader = RunEventReader(
        steps: UInt64(request.draft.settings.steps), fingerprint: fingerprint)
      // FileHandle.read(upToCount:) may wait to fill the buffer. A pipe read
      // returns as soon as an event arrives, even when the engine stays alive.
      var buffer = [UInt8](repeating: 0, count: 16_384)
      while true {
        let count = Darwin.read(pipe.fileHandleForReading.fileDescriptor, &buffer, buffer.count)
        if count < 0 {
          if errno == EINTR { continue }
          throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
        }
        if count == 0 { break }
        let bytes = Data(buffer.prefix(count))
        try eventLog.write(contentsOf: bytes)
        for event in try reader.append(bytes) { onEvent(event) }
      }
      child.waitUntilExit()
      if cancellationRequested && child.terminationReason == .uncaughtSignal {
        result = RunResult(
          status: .cancelled, exitCode: child.terminationStatus, error: nil, details: nil)
      } else {
        try reader.finish()
        if reader.terminal?.event == "failed" {
          result = RunResult(
            status: .failed, exitCode: child.terminationStatus,
            error: "The engine could not complete this analysis.", details: reader.terminal?.message
          )
        } else if reader.terminal?.event == "completed", child.terminationStatus == 0 {
          try verifyRecord(request, status: "completed", step: UInt64(request.draft.settings.steps))
          result = RunResult(status: .completed, exitCode: 0, error: nil, details: nil)
        } else if reader.terminal?.event == "cancelled", child.terminationStatus == 130,
          let step = reader.terminal?.step
        {
          try verifyRecord(request, status: "cancelled", step: step)
          result = RunResult(status: .cancelled, exitCode: 130, error: nil, details: nil)
        } else {
          throw RunFailure.invalidCompletion
        }
      }
    } catch {
      cancel()
      launched?.waitUntilExit()
      result = RunResult(
        status: .failed, exitCode: launched?.terminationStatus,
        error: (error as? RunFailure)?.errorDescription
          ?? "The analysis could not be started or saved.",
        details: error is RunFailure ? nil : error.localizedDescription)
    }
    lock.lock()
    process = nil
    try? input?.close()
    input = nil
    lock.unlock()
    if ownsDirectory {
      do { try writeRecord(request, status: result.status.rawValue, result: result) } catch {
        return RunResult(
          status: .failed, exitCode: result.exitCode,
          error: "The run record could not be saved. Inspect the output folder.",
          details: error.localizedDescription)
      }
    }
    return result
  }

  private func verifyRecord(_ request: RunRequest, status: String, step: UInt64) throws {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let record = try decoder.decode(
      EngineRecord.self, from: Data(contentsOf: request.output.appendingPathComponent("run.json")))
    guard record.status == status, record.completedSteps == step,
      record.input.sha256 == request.draft.matrixSHA256,
      record.settings.steps == UInt64(request.draft.settings.steps),
      record.settings.sampleEvery == UInt64(request.draft.settings.sampleEvery),
      record.settings.seed == UInt64(request.draft.settings.seed),
      record.model.monophyly == request.draft.monophyly
    else { throw RunFailure.invalidCompletion }
    for name in ["trace.log", "trees.nexus"] {
      let values = try request.output.appendingPathComponent(name).resourceValues(forKeys: [
        .isRegularFileKey, .fileSizeKey,
      ])
      guard values.isRegularFile == true, (values.fileSize ?? 0) > 0 else {
        throw RunFailure.invalidCompletion
      }
    }
  }

  private func writeRecord(_ request: RunRequest, status: String, result: RunResult?) throws {
    let record = SessionRecord(
      id: request.id, status: status, setup: request.draft,
      exitCode: result?.exitCode, error: result?.error)
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    encoder.dateEncodingStrategy = .iso8601
    try encoder.encode(record).write(
      to: request.directory.appendingPathComponent("session.json"), options: .atomic)
  }
}
