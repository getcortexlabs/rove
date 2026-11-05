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
      version: "0.1.3",
      headline: "More data, clearer choices.",
      introduction:
        "Read your matrices more closely, combine morphology with DNA, and carry the evidence with your results.",
      highlights: [
        ReleaseHighlight(
          id: "observation-review", symbol: "square.grid.3x3",
          title: "Make every character count.",
          summary: "Review labels, possible states and observations that need attention.",
          detail:
            "More NEXUS state-label exports now open correctly. Inspect constant characters and uncertainty, and choose complete state labels as the alphabet when appropriate. True polymorphism still needs a supported observation model before inference.",
          action: "Try the example", destination: .example),
        ReleaseHighlight(
          id: "dna-mixed", symbol: "point.3.connected.trianglepath.dotted",
          title: "Bring DNA into the picture.",
          summary: "Analyze aligned DNA with JC69 or combine it with morphology.",
          detail:
            "Open aligned FASTA or DNA NEXUS files, or combine one morphological matrix with one DNA alignment. Match taxa explicitly and choose a fixed or estimated morphology-to-DNA rate ratio. The combined model shares its tree, clock and gamma shape.",
          action: "Explore the field guide", destination: .guide),
        ReleaseHighlight(
          id: "scientific-choices", symbol: "slider.horizontal.3",
          title: "See the assumptions behind the tree.",
          summary: "Explore rooting diagrams, sigma-prior curves and the optional Yule prior.",
          detail:
            "See the split imposed by rooting and how Alpha and Beta shape the clock-sigma prior. Yule adds a no-extinction option for contemporaneous species. Birth-death remains the default, and fossil dating keeps its FBD model.",
          action: "Try the example", destination: .example),
        ReleaseHighlight(
          id: "evidence-export", symbol: "square.and.arrow.up",
          title: "Keep the evidence together.",
          summary: "Review sampling and predictive checks, then export a reproducible package.",
          detail:
            "History brings completion, diagnostics, agreement, trees and predictive evidence into one assessment. Paired predictive plots help you inspect discrepancies. Export the supporting files with a manifest, and choose explicitly whether to attach the original matrix.",
          action: "Open History", destination: .history),
      ]
    ),
    RoveRelease(
      version: "0.1.2",
      headline: "A little more room to explore.",
      introduction:
        "More control over your analysis, more ways to open your results, and updates that come to you.",
      highlights: [
        ReleaseHighlight(
          id: "analysis-controls", symbol: "slider.horizontal.3",
          title: "Set your own pace.",
          summary: "Choose your sampling plan and run up to 16 independent chains.",
          detail:
            "Use intermediate presets or enter your own step count and sampling interval. Configure comparison burn-in, execution options and the relaxed-clock sigma prior in the app. Review and History keep those choices with the analysis.",
          action: "Try the example", destination: .example),
        ReleaseHighlight(
          id: "results-tools", symbol: "chart.xyaxis.line",
          title: "Your results, within reach.",
          summary:
            "Open an external trace, import a completed CLI run or compare models from Review.",
          detail:
            "Trace diagnostics accept burn-in as a percentage or a number of samples. Model evidence has its own sampling interval and no longer needs an ordinary analysis first. Imported CLI runs are checked against their original matrix.",
          action: "Open History", destination: .history),
        ReleaseHighlight(
          id: "native-updates", symbol: "arrow.triangle.2.circlepath",
          title: "The next update comes to you.",
          summary: "Check for signed updates from the Rove menu or Settings.",
          detail:
            "This first upgrade from 0.1.1 needs a manual DMG installation. After that, Rove can find and install updates itself. Active calculations delay installation; when your work finishes, you choose when to relaunch.",
          action: "Explore the field guide", destination: .guide),
      ]
    ),
    RoveRelease(
      version: "0.1.1",
      headline: "Trees from anywhere.",
      introduction:
        "A tree sample from BEAST 2 or another program now has a home in Rove: summarize it, explore it and find it again in History.",
      highlights: [
        ReleaseHighlight(
          id: "tree-files", symbol: "point.3.connected.trianglepath.dotted",
          title: "Bring the trees you already have.",
          summary: "Summarize a tree file from another program and explore its MCC tree.",
          detail:
            "Choose a NEXUS tree sample from the welcome screen, the sidebar, File → Summarize Tree File… or the matrix import, which recognizes it. Summaries live in a folder named after the file and History lists it as a tree file. The engine now reads BEAST 2 trees, root stem included.",
          action: "Open History", destination: .history),
        ReleaseHighlight(
          id: "summary-sheet", symbol: "rectangle.and.text.magnifyingglass",
          title: "A summary that fits the window.",
          summary: "No truncated text, whole summary cards, the file named at the top.",
          detail:
            "The summary sheet scrolls above a fixed footer, so the options, the explanation and every saved summary stay visible on a small screen.",
          action: "Explore the field guide", destination: .guide),
      ]
    ),
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
            "Import a NEXUS matrix, review its state partitions and choose the taxon used for rooting explicitly. The bundled 20 × 50 example is a good place to start.",
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
    ),
  ]
}

@MainActor @Observable final class LaunchExperience {
  enum Presentation: String, Identifiable {
    case tour, news
    var id: String { rawValue }
  }
  enum TourAction { case workspace, example, importMatrix }
  enum EntryStage { case beta, dedication, tour, workspace }
  private(set) var completed: Bool
  private(set) var dedicationAcknowledged: Bool
  var ready: Bool { dedicationAcknowledged && completed }
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
    dedicationAcknowledged = defaults.bool(forKey: "dedicationAcknowledged")
    acknowledgedVersion = defaults.string(forKey: "releaseNotesAcknowledged")
  }

  func entryStage(allowed: Bool, running: Bool = false) -> EntryStage {
    if running { return .workspace }
    guard allowed else { return .beta }
    if !dedicationAcknowledged { return .dedication }
    return completed ? .workspace : .tour
  }

  func acknowledgeDedication() {
    dedicationAcknowledged = true
    defaults.set(true, forKey: "dedicationAcknowledged")
  }

  var availableReleases: [RoveRelease] {
    guard let installedVersion, let version = ReleaseVersion(installedVersion) else { return [] }
    return releases.filter { release in
      ReleaseVersion(release.version).map { $0 <= version } ?? false
    }.sorted { ReleaseVersion($0.version)! > ReleaseVersion($1.version)! }
  }

  var currentRelease: RoveRelease? { availableReleases.first { $0.version == installedVersion } }

  func presentNewsIfNeeded(allowed: Bool, idle: Bool) {
    guard allowed, idle, ready, presentation == nil, let current = currentRelease,
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
