import Foundation
import Observation

@MainActor @Observable
final class AnalysisRun {
  let request: RunRequest
  var id: UUID { request.id }
  private(set) var status = RunStatus.preparing
  private(set) var progress: RunEvent?
  private(set) var result: RunResult?
  private(set) var began = Date()
  private(set) var ended: Date?
  private let worker = RunProcess()
  private var task: Task<Void, Never>?
  var isActive: Bool { status.isActive }

  init(request: RunRequest) { self.request = request }

  func start(afterFinish: (@MainActor () async -> Void)? = nil) {
    guard task == nil else { return }
    began = Date()
    let (stream, continuation) = AsyncStream<RunEvent>.makeStream(
      bufferingPolicy: .bufferingNewest(1))
    let request = request
    let worker = worker
    task = Task {
      let work = Task {
        let result = await worker.execute(request) { continuation.yield($0) }
        continuation.finish()
        return result
      }
      for await event in stream { receive(event, runID: request.id) }
      let result = await work.value
      self.result = result
      status = result.status
      ended = Date()
      await afterFinish?()
    }
  }

  func receive(_ event: RunEvent, runID: UUID) {
    guard runID == id, isActive else { return }
    if event.step != nil { progress = event }
    if status == .preparing, event.event != "failed" { status = .running }
  }

  func cancel() {
    guard isActive else { return }
    status = .cancelling
    worker.cancel()
  }

  func waitForExit() async { await task?.value }
}
