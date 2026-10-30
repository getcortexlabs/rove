import Foundation
import Observation

struct ReleaseVersion: Comparable, Equatable {
  let components: [Int]
  init?(_ text: String) {
    let parts = text.split(separator: ".", omittingEmptySubsequences: false)
    guard parts.count == 3 else { return nil }
    let numbers = parts.compactMap { part -> Int? in
      guard !part.isEmpty, part.allSatisfy({ $0.isASCII && $0.isNumber }) else { return nil }
      return Int(part)
    }
    guard numbers.count == 3 else { return nil }
    components = numbers
  }
  static func < (lhs: Self, rhs: Self) -> Bool {
    lhs.components.lexicographicallyPrecedes(rhs.components)
  }
}

struct ReleaseHighlight: Identifiable {
  enum Destination { case example, history, guide }
  let id: String
  let symbol: String
  let title: String
  let summary: String
  let detail: String
  let action: String
  let destination: Destination
}

struct RoveRelease: Identifiable {
  let version: String
  let headline: String
  let introduction: String
  let highlights: [ReleaseHighlight]
  var id: String { version }

  static let all = [
    RoveRelease(
      version: "0.1.0",
      headline: "Your first field notes.",
      introduction:
        "A small beginning, with room for big questions. Meet the tools already inside this beta.",
      highlights: [
        ReleaseHighlight(
          id: "morphology", symbol: "square.grid.3x3", title: "Bring all the little differences.",
          summary: "Morphological matrices, with two, three or more states.",
          detail:
            "Import a NEXUS matrix, review its state partitions and choose the outgroup explicitly. The bundled 20 × 50 example is a good place to start.",
          action: "Try the example", destination: .example),
        ReleaseHighlight(
          id: "history", symbol: "clock.arrow.circlepath",
          title: "Good work deserves a save point.",
          summary: "Find past runs. Pause a long analysis. Pick up where you left off.",
          detail:
            "History keeps the run configuration and output paths together. Checkpoints preserve the sampler state; resuming requires the same engine and original input.",
          action: "Open History", destination: .history),
        ReleaseHighlight(
          id: "evidence", symbol: "point.3.connected.trianglepath.dotted",
          title: "Look closer at the evidence.",
          summary: "Inspect traces, compare chains and explore annotated trees.",
          detail:
            "Review burn-in, ESS and agreement between chains before interpreting a tree. Rove can summarize posterior trees and export an annotated tree or an SVG figure.",
          action: "Explore the field guide", destination: .guide),
      ]
    )
  ]
}

@MainActor @Observable final class LaunchExperience {
  enum Presentation: String, Identifiable {
    case tour, news
    var id: String { rawValue }
  }
  enum TourAction { case workspace, example, importMatrix }
  private(set) var completed: Bool
  var presentation: Presentation?
  var pendingURL: URL?
  private let defaults: UserDefaults
  let installedVersion: String?
  let releases: [RoveRelease]
  private(set) var acknowledgedVersion: String?

  init(
    defaults: UserDefaults = .standard,
    version: String? = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString")
      as? String,
    releases: [RoveRelease] = RoveRelease.all
  ) {
    self.defaults = defaults
    installedVersion = version
    self.releases = releases
    completed = defaults.bool(forKey: "onboardingCompleted")
    acknowledgedVersion = defaults.string(forKey: "releaseNotesAcknowledged")
  }

  var availableReleases: [RoveRelease] {
    guard let installedVersion, let version = ReleaseVersion(installedVersion) else { return [] }
    return releases.filter { release in
      ReleaseVersion(release.version).map { $0 <= version } ?? false
    }.sorted { ReleaseVersion($0.version)! > ReleaseVersion($1.version)! }
  }

  var currentRelease: RoveRelease? { availableReleases.first { $0.version == installedVersion } }

  func presentNewsIfNeeded(allowed: Bool, idle: Bool) {
    guard allowed, idle, completed, presentation == nil, let current = currentRelease,
      let version = ReleaseVersion(current.version)
    else { return }
    if let seen = acknowledgedVersion.flatMap(ReleaseVersion.init), seen >= version { return }
    presentation = .news
  }

  func completeTour() {
    completed = true
    defaults.set(true, forKey: "onboardingCompleted")
    // First launch already introduces the installed features. Do not stack
    // a second announcement immediately after the tour.
    acknowledgeNews()
    presentation = nil
  }

  func acknowledgeNews() {
    guard let current = currentRelease, let version = ReleaseVersion(current.version) else {
      return
    }
    if let seen = acknowledgedVersion.flatMap(ReleaseVersion.init), seen > version { return }
    acknowledgedVersion = current.version
    defaults.set(current.version, forKey: "releaseNotesAcknowledged")
  }

  func dismissNews() {
    acknowledgeNews()
    presentation = nil
  }
}
