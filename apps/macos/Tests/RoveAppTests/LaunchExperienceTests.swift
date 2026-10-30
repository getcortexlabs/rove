import Foundation
import Testing

@testable import RoveApp

@MainActor struct LaunchExperienceTests {
  private func defaults() throws -> UserDefaults {
    let name = "rove-launch-tests-\(UUID().uuidString)"
    let result = try #require(UserDefaults(suiteName: name))
    result.removePersistentDomain(forName: name)
    return result
  }

  @Test func firstLaunchFinishesWithoutStackingAnnouncements() throws {
    let storage = try defaults()
    let experience = LaunchExperience(defaults: storage, version: "0.1.0")
    #expect(!experience.completed)
    experience.presentNewsIfNeeded(allowed: true, idle: true)
    #expect(experience.presentation == nil)
    experience.acknowledgeDedication()
    experience.completeTour()
    experience.presentNewsIfNeeded(allowed: true, idle: true)
    #expect(experience.presentation == nil)
    let restored = LaunchExperience(defaults: storage, version: "0.1.0")
    #expect(restored.completed)
    #expect(restored.acknowledgedVersion == "0.1.0")
    restored.presentation = .tour
    #expect(restored.completed)
  }

  @Test func updateWaitsForAccessAndIdleWorkspaceAndAppearsOnce() throws {
    let storage = try defaults()
    storage.set(true, forKey: "onboardingCompleted")
    storage.set("0.0.9", forKey: "releaseNotesAcknowledged")
    let experience = LaunchExperience(defaults: storage, version: "0.1.0")
    experience.acknowledgeDedication()
    experience.presentNewsIfNeeded(allowed: false, idle: true)
    #expect(experience.presentation == nil)
    experience.presentNewsIfNeeded(allowed: true, idle: false)
    #expect(experience.presentation == nil)
    experience.presentation = .tour
    experience.presentNewsIfNeeded(allowed: true, idle: true)
    #expect(experience.presentation == .tour)
    experience.presentation = nil
    experience.presentNewsIfNeeded(allowed: true, idle: true)
    #expect(experience.presentation == .news)
    experience.dismissNews()
    experience.presentNewsIfNeeded(allowed: true, idle: true)
    #expect(experience.presentation == nil)
  }

  @Test func dedicationWaitsForAccessAndPersistsBeforeTheTour() throws {
    let storage = try defaults()
    let experience = LaunchExperience(defaults: storage, version: "0.1.0")
    let matrix = URL(fileURLWithPath: "/tmp/pending-matrix.nex")
    experience.pendingURL = matrix
    #expect(experience.entryStage(allowed: false) == .beta)
    #expect(experience.entryStage(allowed: true) == .dedication)
    #expect(!experience.ready)
    // Merely opening the letter does not consume it if the app closes.
    let reopened = LaunchExperience(defaults: storage, version: "0.1.0")
    #expect(reopened.entryStage(allowed: true) == .dedication)
    experience.acknowledgeDedication()
    #expect(experience.pendingURL == matrix)
    #expect(experience.entryStage(allowed: true) == .tour)
    #expect(!experience.completed)
    let restored = LaunchExperience(defaults: storage, version: "0.2.0")
    #expect(restored.entryStage(allowed: false) == .beta)
    #expect(restored.entryStage(allowed: true) == .tour)
    restored.completeTour()
    #expect(restored.ready)
    #expect(restored.entryStage(allowed: true) == .workspace)
  }

  @Test func existingUsersSeeTheLetterWithoutRepeatingTheirTour() throws {
    let storage = try defaults()
    storage.set(true, forKey: "onboardingCompleted")
    storage.set("0.0.9", forKey: "releaseNotesAcknowledged")
    let experience = LaunchExperience(defaults: storage, version: "0.1.0")
    #expect(experience.entryStage(allowed: true) == .dedication)
    experience.presentNewsIfNeeded(allowed: true, idle: true)
    #expect(experience.presentation == nil)
    #expect(experience.entryStage(allowed: false, running: true) == .workspace)
    #expect(!experience.dedicationAcknowledged)
    experience.acknowledgeDedication()
    #expect(experience.entryStage(allowed: true) == .workspace)
    experience.presentNewsIfNeeded(allowed: true, idle: true)
    #expect(experience.presentation == .news)
    let restored = LaunchExperience(defaults: storage, version: "0.1.0")
    #expect(restored.ready)
    #expect(restored.entryStage(allowed: true) == .workspace)
    restored.presentation = .tour
    restored.completeTour()
    #expect(restored.dedicationAcknowledged)
  }

  @Test func versionsCompareNumericallyAndUnknownBuildsDoNotInventNews() throws {
    #expect(ReleaseVersion("0.10.0")! > ReleaseVersion("0.9.9")!)
    #expect(ReleaseVersion("1.0") == nil)
    #expect(ReleaseVersion("1.0.0-beta") == nil)
    let storage = try defaults()
    storage.set(true, forKey: "onboardingCompleted")
    storage.set("0.10.0", forKey: "releaseNotesAcknowledged")
    let downgrade = LaunchExperience(defaults: storage, version: "0.1.0")
    downgrade.acknowledgeDedication()
    downgrade.presentNewsIfNeeded(allowed: true, idle: true)
    downgrade.acknowledgeNews()
    #expect(downgrade.presentation == nil)
    #expect(downgrade.acknowledgedVersion == "0.10.0")
    for version in [nil, "invalid", "0.0.1", "0.2.0"] as [String?] {
      let experience = LaunchExperience(defaults: storage, version: version)
      experience.presentNewsIfNeeded(allowed: true, idle: true)
      #expect(experience.currentRelease == nil)
      #expect(experience.presentation == nil)
    }
  }
}
