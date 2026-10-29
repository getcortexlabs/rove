import CryptoKit
import Darwin
import Foundation

struct RunRequest: Sendable {
  let id: UUID
  let executable: URL
  let directory: URL
  let draft: AnalysisDraft
  var resume = false
  var output: URL { directory.appendingPathComponent("results") }
  var arguments: [String] {
    if resume {
      return [
        "sample", "--resume", output.path, "--events-json", "--cancel-on-stdin-close",
        "--expect-input-sha256", draft.matrixSHA256,
      ]
    }
    return draft.settings.arguments(
      matrix: URL(fileURLWithPath: draft.matrixPath), output: output,
      monophyly: draft.constraints) + [
        "--events-json", "--cancel-on-stdin-close", "--expect-input-sha256", draft.matrixSHA256,
        "--print-every", String(max(1, min(100_000, draft.settings.steps / 100))),
      ]
  }
}

enum RunStatus: String, Codable, Sendable {
  case preparing, running, pausing, cancelling, completed, paused, cancelled, failed
  var isActive: Bool {
    self == .preparing || self == .running || self == .pausing || self == .cancelling
  }
}

struct RunResult: Sendable {
  let status: RunStatus
  let exitCode: Int32?
  let error: String?
  let details: String?
}

// The worker owns all blocking I/O. The lock only protects process ownership
// and cancellation shared with the UI; it is never held while waiting for exit.
final class RunProcess: @unchecked Sendable {
  private enum Control: Error { case cancelled }
  private let lock = NSLock()
  private var process: Process?
  private var input: FileHandle?
  private var requestedCancellation = false
  private var requestedPause = false

  func pause() {
    lock.lock()
    defer { lock.unlock() }
    requestedPause = true
    try? input?.write(contentsOf: Data("pause\n".utf8))
  }

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

  func execute(_ request: RunRequest, onEvent: @escaping @Sendable (RunEvent) -> Void) async
    -> RunResult
  {
    // Pipe reads and process waits must not occupy Swift's cooperative executor.
    await withCheckedContinuation { continuation in
      Thread.detachNewThread {
        autoreleasepool {
          continuation.resume(returning: self.executeBlocking(request, onEvent: onEvent))
        }
      }
    }
  }

  private func executeBlocking(_ request: RunRequest, onEvent: @Sendable (RunEvent) -> Void)
    -> RunResult
  {
    var result: RunResult
    var launched: Process?
    var ownsDirectory = false
    let previous =
      request.resume
      ? try? SessionRecord.read(request.directory.appendingPathComponent("session.json")) : nil
    let began = Date()
    var sessionLock: FileHandle?
    defer { try? sessionLock?.close() }
    var resumeStep: UInt64?
    var lastStep: UInt64?
    do {
      try request.draft.validate()
      try FileManager.default.createDirectory(
        at: request.directory.deletingLastPathComponent(), withIntermediateDirectories: true)
      if request.resume {
        let checkpoint = try RunCheckpoint.read(directory: request.directory, draft: request.draft)
        resumeStep = checkpoint.step
        lastStep = checkpoint.step
      } else {
        try FileManager.default.createDirectory(
          at: request.directory, withIntermediateDirectories: false)
        ownsDirectory = true
      }
      let lockPath = request.directory.appendingPathComponent(".session.lock")
      let descriptor = Darwin.open(lockPath.path, O_CREAT | O_RDWR | O_CLOEXEC, S_IRUSR | S_IWUSR)
      guard descriptor >= 0 else { throw RunFailure.runInUse }
      sessionLock = FileHandle(fileDescriptor: descriptor, closeOnDealloc: true)
      guard flock(descriptor, LOCK_EX | LOCK_NB) == 0 else { throw RunFailure.runInUse }
      if !request.resume {
        try request.draft.write(to: request.directory.appendingPathComponent("setup.rove.json"))
        try writeRecord(request, status: "partial", result: nil, began: began, step: nil)
      }
      if cancellationRequested {
        lastStep = 0
        throw Control.cancelled
      }
      let matrix = URL(fileURLWithPath: request.draft.matrixPath)
      let values = try matrix.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
      guard values.isRegularFile == true, (values.fileSize ?? 0) <= 32 * 1024 * 1024 else {
        throw RunFailure.changedInput
      }
      let fingerprint = SHA256.hash(data: try Data(contentsOf: matrix)).map {
        String(format: "%02x", $0)
      }.joined()
      guard fingerprint == request.draft.matrixSHA256 else { throw RunFailure.changedInput }
      let errors = request.directory.appendingPathComponent("engine.stderr.log")
      let events = request.directory.appendingPathComponent("events.jsonl")
      if !request.resume {
        FileManager.default.createFile(atPath: errors.path, contents: nil)
        FileManager.default.createFile(atPath: events.path, contents: nil)
      } else {
        for url in [errors, events] where !FileManager.default.fileExists(atPath: url.path) {
          FileManager.default.createFile(atPath: url.path, contents: nil)
        }
      }
      let stderr = try FileHandle(forWritingTo: errors)
      let eventLog = try FileHandle(forWritingTo: events)
      if request.resume {
        try stderr.seekToEnd()
        try eventLog.seekToEnd()
      }
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
      let pausedBeforeLaunch = requestedPause
      lock.unlock()
      launched = child
      try? owner.fileHandleForReading.close()
      try? pipe.fileHandleForWriting.close()
      if cancelledBeforeLaunch { cancel() } else if pausedBeforeLaunch { pause() }
      var reader = RunEventReader(
        steps: UInt64(request.draft.settings.steps), fingerprint: fingerprint,
        chains: request.draft.settings.chains, resumedStep: resumeStep)
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
        for event in try reader.append(bytes) {
          if request.resume && !ownsDirectory && event.event == "started" {
            ownsDirectory = true
            try writeRecord(
              request, status: "partial", result: nil, began: previous?.startedAt ?? began,
              step: resumeStep)
          }
          lastStep = event.step ?? lastStep
          onEvent(event)
        }
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
        } else if ["completed", "group_completed"].contains(reader.terminal?.event ?? ""),
          child.terminationStatus == 0
        {
          try verifyRecord(request, status: "completed", step: reader.terminal?.step ?? 0)
          result = RunResult(status: .completed, exitCode: 0, error: nil, details: nil)
        } else if ["paused", "group_paused"].contains(reader.terminal?.event ?? ""),
          child.terminationStatus == 130, let step = reader.terminal?.step
        {
          try verifyRecord(request, status: "paused", step: step)
          let checkpoint = try RunCheckpoint.read(
            directory: request.directory, draft: request.draft)
          guard checkpoint.step == step else { throw RunFailure.invalidCheckpoint }
          result = RunResult(status: .paused, exitCode: 130, error: nil, details: nil)
        } else if ["cancelled", "group_cancelled"].contains(reader.terminal?.event ?? ""),
          child.terminationStatus == 130,
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
      if error is Control {
        result = RunResult(status: .cancelled, exitCode: nil, error: nil, details: nil)
      } else {
        result = RunResult(
          status: .failed, exitCode: launched?.terminationStatus,
          error: (error as? RunFailure)?.errorDescription
            ?? "The analysis could not be started or saved.",
          details: error is RunFailure ? nil : error.localizedDescription)
      }
    }
    lock.lock()
    process = nil
    try? input?.close()
    input = nil
    lock.unlock()
    if ownsDirectory {
      do {
        try writeRecord(
          request, status: result.status.rawValue, result: result,
          began: previous?.startedAt ?? began, step: lastStep,
          elapsed: (previous?.elapsedSeconds ?? 0) + Date().timeIntervalSince(began))
      } catch {
        return RunResult(
          status: .failed, exitCode: result.exitCode,
          error: "The run record could not be saved. Inspect the output folder.",
          details: error.localizedDescription)
      }
    }
    return result
  }

  private func verifyRecord(_ request: RunRequest, status: String, step: UInt64) throws {
    let record = try EngineRecord.verify(directory: request.directory, draft: request.draft)
    guard record.status == status, record.completedSteps == step else {
      throw RunFailure.invalidCompletion
    }
  }

  private func writeRecord(
    _ request: RunRequest, status: String, result: RunResult?, began: Date, step: UInt64?,
    elapsed: Double? = nil
  ) throws {
    let record = SessionRecord(
      formatVersion: 1, id: request.id, status: status, setup: request.draft,
      exitCode: result?.exitCode, error: result?.error, startedAt: began,
      finishedAt: result == nil ? nil : Date(),
      elapsedSeconds: result == nil ? nil : (elapsed ?? Date().timeIntervalSince(began)),
      completedSteps: step)
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    encoder.dateEncodingStrategy = .iso8601
    try encoder.encode(record).write(
      to: request.directory.appendingPathComponent("session.json"), options: .atomic)
  }
}
