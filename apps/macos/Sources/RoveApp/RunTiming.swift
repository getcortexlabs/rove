import Foundation

/// A recent speed estimate from engine active time, independent of UI refreshes.
struct RunTiming: Sendable {
  struct Estimate: Sendable {
    let remainingSeconds: Double
    let secondsPerMillion: Double
  }

  private struct Sample: Sendable {
    let step: UInt64
    let seconds: Double
  }
  private var samples: [Sample] = []
  private var totalSteps: UInt64 = 0

  mutating func receive(_ event: RunEvent) {
    guard let step = event.step, let total = event.totalSteps,
      let seconds = event.elapsedSeconds, seconds.isFinite, seconds >= 0, step <= total
    else { return }
    if event.event == "started" { samples.removeAll() }
    if let previous = samples.last {
      guard step > previous.step, seconds > previous.seconds, total == totalSteps else { return }
      // Fast matrices can emit many updates per second. Aggregate them so the
      // bounded window still spans the learning period instead of staying cold.
      guard seconds - previous.seconds >= 2 || step == total else { return }
    }
    totalSteps = total
    samples.append(Sample(step: step, seconds: seconds))
    // A bounded window smooths bursts while adapting to changes in CPU load.
    if samples.count > 9 { samples.removeFirst(samples.count - 9) }
  }

  var estimate: Estimate? {
    guard samples.count >= 4, let first = samples.first, let last = samples.last,
      last.seconds - first.seconds >= 10, last.step > first.step
    else { return nil }
    let secondsPerStep = (last.seconds - first.seconds) / Double(last.step - first.step)
    let remaining = Double(totalSteps - last.step) * secondsPerStep
    let perMillion = secondsPerStep * 1_000_000
    guard remaining.isFinite, perMillion.isFinite else { return nil }
    return Estimate(remainingSeconds: remaining, secondsPerMillion: perMillion)
  }

  static func remaining(chains: [RunTiming]) -> Double? {
    let estimates = chains.compactMap { chain -> Double? in
      if chain.samples.last?.step == chain.totalSteps { return 0 }
      return chain.estimate?.remainingSeconds
    }
    guard !chains.isEmpty, estimates.count == chains.count else { return nil }
    return estimates.max()
  }
}
