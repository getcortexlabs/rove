import Foundation

/// Runs one engine command whose standard input the app owns: closing it
/// cancels the engine, which then removes or marks its partial output and
/// exits with a failure status. An engine that finishes before noticing the
/// closed input has committed its output, so cancellation is reported only
/// with a failure status. Standard error reaches `onLine` line by line; the
/// last lines are kept for error reports.
final class EngineTool: @unchecked Sendable {
  struct Outcome: Sendable {
    let status: Int32
    let log: String
  }

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

  func execute(
    executable: URL, arguments: [String], onLine: @escaping @Sendable (String) -> Void
  ) async throws -> Outcome {
    try await withCheckedThrowingContinuation { continuation in
      Thread.detachNewThread {
        autoreleasepool {
          continuation.resume(
            with: Result {
              try self.run(executable: executable, arguments: arguments, onLine: onLine)
            })
        }
      }
    }
  }

  private func run(executable: URL, arguments: [String], onLine: (String) -> Void) throws
    -> Outcome
  {
    let errors = Pipe()
    let owner = Pipe()
    let child = Process()
    child.executableURL = executable
    child.arguments = arguments
    child.standardInput = owner
    child.standardOutput = FileHandle.nullDevice
    child.standardError = errors
    lock.lock()
    if cancelled {
      lock.unlock()
      throw CancellationError()
    }
    do { try child.run() } catch {
      lock.unlock()
      throw error
    }
    input = owner.fileHandleForWriting
    lock.unlock()
    try? owner.fileHandleForReading.close()
    try? errors.fileHandleForWriting.close()
    var pending = Data()
    var tail = ""
    var buffer = [UInt8](repeating: 0, count: 16_384)
    let descriptor = errors.fileHandleForReading.fileDescriptor
    // A pipe read returns as soon as the engine writes, unlike
    // FileHandle.read(upToCount:), which may wait to fill its buffer.
    while true {
      let count = Darwin.read(descriptor, &buffer, buffer.count)
      if count < 0 {
        if errno == EINTR { continue }
        break
      }
      if count == 0 { break }
      pending.append(contentsOf: buffer.prefix(count))
      while let newline = pending.firstIndex(of: UInt8(ascii: "\n")) {
        let line = String(decoding: pending[pending.startIndex..<newline], as: UTF8.self)
        pending.removeSubrange(pending.startIndex...newline)
        onLine(line)
        tail = String((tail + line + "\n").suffix(16_384))
      }
    }
    if !pending.isEmpty {
      let line = String(decoding: pending, as: UTF8.self)
      onLine(line)
      tail = String((tail + line).suffix(16_384))
    }
    try? errors.fileHandleForReading.close()
    child.waitUntilExit()
    lock.lock()
    try? input?.close()
    input = nil
    let stopped = cancelled
    lock.unlock()
    if stopped && child.terminationStatus != 0 { throw CancellationError() }
    return Outcome(
      status: child.terminationStatus,
      log: tail.trimmingCharacters(in: .whitespacesAndNewlines))
  }
}
