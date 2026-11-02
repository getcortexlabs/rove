import Foundation
import Testing

@testable import RoveApp

struct RunTimingTests {
  private func event(_ step: UInt64, _ seconds: Double, kind: String = "progress") -> RunEvent {
    RunEvent(
      schemaVersion: 1, event: kind, step: step, totalSteps: 1_000_000,
      elapsedSeconds: seconds, inputSha256: nil, posterior: nil, likelihood: nil, prior: nil,
      message: nil)
  }

  @Test func learnsFromDeltasThenAdaptsToRecentSpeed() throws {
    var timing = RunTiming()
    timing.receive(event(0, 0, kind: "started"))
    for index in 1...2 { timing.receive(event(UInt64(index * 10_000), Double(index * 5))) }
    #expect(timing.estimate == nil)
    timing.receive(event(30_000, 15))
    #expect(try #require(timing.estimate).secondsPerMillion == 500)
    #expect(try #require(timing.estimate).remainingSeconds == 485)
    // A slower recent window replaces the initial rate without storing an entire run.
    for index in 1...9 {
      timing.receive(event(UInt64(30_000 + index * 10_000), Double(15 + index * 10)))
    }
    #expect(try #require(timing.estimate).secondsPerMillion == 1_000)
  }

  @Test func resumedRunLearnsAgainWithoutCountingOldWorkOrPausedTime() throws {
    var timing = RunTiming()
    for index in 0...3 { timing.receive(event(UInt64(index * 10_000), Double(index * 5))) }
    timing.receive(event(500_000, 250, kind: "started"))
    #expect(timing.estimate == nil)
    for index in 1...3 {
      timing.receive(event(UInt64(500_000 + index * 10_000), Double(250 + index * 10)))
    }
    #expect(try #require(timing.estimate).secondsPerMillion == 1_000)
    #expect(try #require(timing.estimate).remainingSeconds == 470)
    timing.receive(event(1, .infinity))
    timing.receive(event(100, 100))
    #expect(try #require(timing.estimate).remainingSeconds == 470)
  }

  @Test func groupWaitsForEveryChainAndUsesTheLongestRemainingTime() {
    var fast = RunTiming()
    var slow = RunTiming()
    for index in 0...3 {
      fast.receive(event(UInt64(index * 100_000), Double(index * 10)))
      slow.receive(event(UInt64(index * 10_000), Double(index * 10)))
    }
    #expect(RunTiming.remaining(chains: [fast, RunTiming()]) == nil)
    #expect(RunTiming.remaining(chains: [fast, slow]) == 970)
    var complete = RunTiming()
    complete.receive(event(1_000_000, 1, kind: "completed"))
    #expect(RunTiming.remaining(chains: [complete, slow]) == 970)
    #expect(RunTiming.remaining(chains: [complete]) == 0)
  }

  @Test func frequentEventsStillFinishLearning() throws {
    var timing = RunTiming()
    for index in 0...200 {
      timing.receive(event(UInt64(index * 1_000), Double(index) / 10))
    }
    #expect(try #require(timing.estimate).secondsPerMillion == 100)
    #expect(try #require(timing.estimate).remainingSeconds == 80)
  }
}
