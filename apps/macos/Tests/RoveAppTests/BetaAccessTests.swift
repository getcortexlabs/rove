import Foundation
import Testing

@testable import RoveApp

@MainActor final class MemoryBetaStore: BetaSessionStore {
  var value: BetaSession?
  var fails = false
  init(_ value: BetaSession? = nil) { self.value = value }
  func load() throws -> BetaSession? { value }
  func save(_ session: BetaSession) throws {
    if fails { throw BetaError.keychain }
    value = session
  }
  func delete() throws { value = nil }
}

actor TestBetaClient: BetaClient {
  let session: BetaSession
  var failure: BetaError?
  var waitlist = false
  var verificationCount = 0
  var requestedEmail: String?
  init(session: BetaSession, failure: BetaError? = nil) {
    self.session = session
    self.failure = failure
  }
  func setFailure(_ failure: BetaError?) { self.failure = failure }
  func request(email: String, purpose: BetaPurpose, language: String) async throws -> BetaChallenge
  {
    if let failure { throw failure }
    requestedEmail = email
    waitlist = purpose == .waitlist
    return BetaChallenge(
      challengeId: "test-challenge", expiresAt: session.verifiedAt.addingTimeInterval(600),
      resendAfter: 60)
  }
  func verify(challenge: String, code: String) async throws -> BetaVerification {
    verificationCount += 1
    if let failure { throw failure }
    return waitlist ? .waitlisted : .authorized(session)
  }
  func refresh(token: String) async throws -> BetaSession {
    if let failure { throw failure }
    return session
  }
  func signOut(token: String) async {}
}

@MainActor struct BetaAccessTests {
  static let date = Date(timeIntervalSince1970: 1_800_000_000)
  static func session(at date: Date = date) -> BetaSession {
    BetaSession(
      token: String(repeating: "a", count: 64), email: "explorer@example.com",
      expiresAt: date.addingTimeInterval(30 * 86400), verifiedAt: date,
      offlineUntil: date.addingTimeInterval(7 * 86400))
  }

  @Test func invitationRequiresVerificationAndUsesKeychainStore() async {
    let client = TestBetaClient(session: Self.session())
    let store = MemoryBetaStore()
    let access = BetaAccess(client: client, store: store, now: { Self.date })
    await access.restore()
    #expect(access.phase == .entry)
    #expect(!access.canUse)
    await access.request(email: " Explorer@Example.COM ", language: "en")
    #expect(await client.requestedEmail == "explorer@example.com")
    #expect(access.phase == .code)
    #expect(!access.canUse)
    await access.verify(code: "12x")
    #expect(await client.verificationCount == 0)
    await access.verify(code: "1234 5678")
    #expect(access.canUse)
    #expect(store.value == Self.session())
    access.signOut()
    #expect(!access.canUse)
    #expect(store.value == nil)
  }

  @Test func confirmedInterestDoesNotAuthorizeTheApp() async {
    let store = MemoryBetaStore()
    let access = BetaAccess(
      client: TestBetaClient(session: Self.session()), store: store, now: { Self.date })
    await access.restore()
    access.begin(.waitlist)
    await access.request(email: "explorer@example.com", language: "pt-BR")
    await access.verify(code: "12345678")
    #expect(access.phase == .waitlisted)
    #expect(!access.canUse)
    #expect(store.value == nil)
  }

  @Test func offlineAccessEndsAfterSevenDaysAndRejectsClockRollback() async {
    let session = Self.session()
    #expect(session.permitsOfflineUse(at: Self.date.addingTimeInterval(7 * 86400 - 1)))
    #expect(!session.permitsOfflineUse(at: Self.date.addingTimeInterval(7 * 86400)))
    #expect(!session.permitsOfflineUse(at: Self.date.addingTimeInterval(-301)))
    for (offset, allowed) in [(1.0, true), (7.0 * 86400, false)] {
      let access = BetaAccess(
        client: TestBetaClient(session: session, failure: .unavailable),
        store: MemoryBetaStore(session), now: { Self.date.addingTimeInterval(offset) })
      await access.restore()
      #expect(access.canUse == allowed)
    }
  }

  @Test func revokedInvitationDoesNotFallBackToOfflineAccess() async {
    let store = MemoryBetaStore(Self.session())
    let access = BetaAccess(
      client: TestBetaClient(session: Self.session(), failure: .signInRequired),
      store: store, now: { Self.date })
    await access.restore()
    #expect(!access.canUse)
    #expect(access.phase == .entry)
    #expect(store.value == nil)
  }

  @Test func failedKeychainWriteNeverUnlocksApp() async {
    let store = MemoryBetaStore()
    store.fails = true
    let access = BetaAccess(
      client: TestBetaClient(session: Self.session()), store: store, now: { Self.date })
    await access.request(email: "explorer@example.com", language: "en")
    await access.verify(code: "12345678")
    #expect(!access.canUse)
    #expect(access.error == BetaError.keychain.message)
  }

  @Test func accessExpiryBlocksNewRunsButLetsActiveAnalysisFinish() async throws {
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let engine =
      ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: folder) }
    let suite = "rove-access-tests-\(UUID().uuidString)"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    var allowed = false
    let workspace = Workspace(
      executable: engine, preferences: AppPreferences(defaults: defaults),
      history: RunHistory(defaults: defaults, defaultRoot: folder), accessAllowed: { allowed })
    workspace.file = matrix
    workspace.report = report
    workspace.page = .review
    workspace.outputParent = folder
    workspace.settings.steps = 2_000
    workspace.settings.sampleEvery = 100
    #expect(workspace.issue == nil)
    #expect(!workspace.canStartRun)
    workspace.startRun()
    #expect(workspace.run == nil)
    allowed = true
    #expect(workspace.canStartRun)
    workspace.startRun()
    let run = try #require(workspace.run)
    allowed = false
    #expect(workspace.isRunning)
    await run.waitForExit()
    #expect(run.status == .completed)
  }
}
