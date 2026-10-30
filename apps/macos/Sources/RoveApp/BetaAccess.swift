import Foundation
import Observation
import Security

enum BetaPurpose: String, Codable, Sendable {
  case access, waitlist
}

struct BetaChallenge: Decodable, Sendable {
  let challengeId: String
  let expiresAt: Date
  let resendAfter: TimeInterval
}

struct BetaSession: Codable, Equatable, Sendable {
  let token: String
  let email: String
  let expiresAt: Date
  let verifiedAt: Date
  let offlineUntil: Date

  func permitsOfflineUse(at date: Date) -> Bool {
    date >= verifiedAt.addingTimeInterval(-300)
      && date < offlineUntil && date < expiresAt
      && offlineUntil <= verifiedAt.addingTimeInterval(7 * 86400)
  }
}

enum BetaVerification: Sendable {
  case authorized(BetaSession)
  case waitlisted
}

enum BetaError: Error, Equatable {
  case invalidEmail, invalidCode, invitationRequired, signInRequired, tryLater
  case unavailable, emailUnavailable, keychain

  var message: String {
    switch self {
    case .invalidEmail: "Enter a valid email address."
    case .invalidCode: "That code is incorrect or has expired. Try again or request a new one."
    case .invitationRequired: "Your invitation is not active. You can join the waiting list."
    case .signInRequired: "Please confirm your beta invitation again."
    case .tryLater: "A few too many attempts. Please wait before trying again."
    case .unavailable: "Connect to the internet to check your beta access, then try again."
    case .emailUnavailable: "We couldn't send the email right now. Please try again shortly."
    case .keychain:
      "Rove couldn't save your access in Keychain. Check your Mac's Keychain access and try again."
    }
  }
}

protocol BetaClient: Sendable {
  func request(email: String, purpose: BetaPurpose, language: String) async throws -> BetaChallenge
  func verify(challenge: String, code: String) async throws -> BetaVerification
  func refresh(token: String) async throws -> BetaSession
  func signOut(token: String) async
}

struct BetaHTTPClient: BetaClient {
  var baseURL = URL(string: "https://getcortexlabs.com/api/rove/beta/")!
  var transport: URLSession = {
    let configuration = URLSessionConfiguration.ephemeral
    configuration.timeoutIntervalForRequest = 15
    configuration.httpShouldSetCookies = false
    configuration.urlCache = nil
    return URLSession(configuration: configuration)
  }()

  private struct VerificationReply: Decodable {
    let status: String
  }

  private func post(_ action: String, body: [String: String] = [:], token: String? = nil)
    async throws -> Data
  {
    var request = URLRequest(url: baseURL.appendingPathComponent(action))
    request.httpMethod = "POST"
    request.setValue("application/json", forHTTPHeaderField: "Content-Type")
    if let token { request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization") }
    request.httpBody = try JSONEncoder().encode(body)
    let (data, response) = try await transport.data(for: request)
    guard let response = response as? HTTPURLResponse, data.count < 8192 else {
      throw BetaError.unavailable
    }
    guard (200..<300).contains(response.statusCode) else {
      let code = (try? JSONDecoder().decode([String: String].self, from: data))?["error"]
      switch code {
      case "invalid_code": throw BetaError.invalidCode
      case "invalid_request": throw BetaError.invalidEmail
      case "invitation_required": throw BetaError.invitationRequired
      case "sign_in_required": throw BetaError.signInRequired
      case "try_later": throw BetaError.tryLater
      case "email_unavailable": throw BetaError.emailUnavailable
      default:
        if response.statusCode == 401 || response.statusCode == 403 {
          throw BetaError.signInRequired
        }
        throw BetaError.unavailable
      }
    }
    return data
  }

  private func decode<T: Decodable>(_ type: T.Type, from data: Data) throws -> T {
    let decoder = JSONDecoder()
    decoder.dateDecodingStrategy = .secondsSince1970
    return try decoder.decode(type, from: data)
  }

  func request(email: String, purpose: BetaPurpose, language: String) async throws -> BetaChallenge
  {
    try await decode(
      BetaChallenge.self,
      from: post(
        "request",
        body: [
          "email": email, "purpose": purpose.rawValue, "language": language,
        ]))
  }

  func verify(challenge: String, code: String) async throws -> BetaVerification {
    let data = try await post("verify", body: ["challengeId": challenge, "code": code])
    switch try decode(VerificationReply.self, from: data).status {
    case "waitlisted": return .waitlisted
    case "authorized": return .authorized(try decode(BetaSession.self, from: data))
    default: throw BetaError.unavailable
    }
  }

  func refresh(token: String) async throws -> BetaSession {
    try await decode(BetaSession.self, from: post("session", token: token))
  }

  func signOut(token: String) async { _ = try? await post("signout", token: token) }
}

@MainActor protocol BetaSessionStore {
  func load() throws -> BetaSession?
  func save(_ session: BetaSession) throws
  func delete() throws
}

struct BetaKeychain: BetaSessionStore {
  private var query: [String: Any] {
    [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: "com.getcortexlabs.rove.beta",
      kSecAttrAccount as String: "session",
    ]
  }

  func load() throws -> BetaSession? {
    var query = query
    query[kSecReturnData as String] = true
    query[kSecMatchLimit as String] = kSecMatchLimitOne
    var value: CFTypeRef?
    let result = SecItemCopyMatching(query as CFDictionary, &value)
    if result == errSecItemNotFound { return nil }
    guard result == errSecSuccess, let data = value as? Data else { throw BetaError.keychain }
    return try JSONDecoder().decode(BetaSession.self, from: data)
  }

  func save(_ session: BetaSession) throws {
    let data = try JSONEncoder().encode(session)
    let result = SecItemUpdate(
      query as CFDictionary, [kSecValueData as String: data] as CFDictionary)
    if result == errSecSuccess { return }
    guard result == errSecItemNotFound else { throw BetaError.keychain }
    var item = query
    item[kSecValueData as String] = data
    item[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
    guard SecItemAdd(item as CFDictionary, nil) == errSecSuccess else { throw BetaError.keychain }
  }

  func delete() throws {
    let result = SecItemDelete(query as CFDictionary)
    guard result == errSecSuccess || result == errSecItemNotFound else { throw BetaError.keychain }
  }
}

@MainActor @Observable final class BetaAccess {
  enum Phase { case checking, entry, code, waitlisted, authorized }
  private(set) var phase = Phase.checking
  private(set) var session: BetaSession?
  private(set) var challenge: BetaChallenge?
  private(set) var purpose = BetaPurpose.access
  private(set) var email = ""
  private(set) var busy = false
  private(set) var refreshing = false
  private(set) var resendAt = Date.distantPast
  var error: String?
  private let client: any BetaClient
  private let store: any BetaSessionStore
  private let now: () -> Date
  private var revision = 0
  private var lastCheck = Date.distantPast

  init(
    client: any BetaClient = BetaHTTPClient(), store: any BetaSessionStore = BetaKeychain(),
    now: @escaping () -> Date = Date.init
  ) {
    self.client = client
    self.store = store
    self.now = now
  }

  var canUse: Bool { phase == .authorized && session?.permitsOfflineUse(at: now()) == true }

  func restore() async {
    guard phase == .checking, !refreshing else { return }
    do { session = try store.load() } catch { self.error = BetaError.keychain.message }
    guard session != nil else {
      phase = .entry
      return
    }
    await refresh()
  }

  func refresh() async {
    guard !refreshing, !busy, let saved = session else { return }
    refreshing = true
    let operation = revision
    defer { refreshing = false }
    do {
      let updated = try await client.refresh(token: saved.token)
      guard revision == operation else { return }
      try accept(updated)
    } catch {
      guard revision == operation else { return }
      if error as? BetaError == .signInRequired || error as? BetaError == .invitationRequired {
        try? store.delete()
        session = nil
        phase = .entry
        self.error = BetaError.signInRequired.message
      } else if saved.permitsOfflineUse(at: now()) {
        phase = .authorized
      } else {
        phase = .entry
        self.error = BetaError.unavailable.message
      }
    }
    lastCheck = now()
  }

  func checkIfNeeded() async {
    guard phase == .authorized || phase == .checking else { return }
    if session != nil && (now().timeIntervalSince(lastCheck) > 6 * 3600 || !canUse) {
      await refresh()
    }
  }

  func begin(_ purpose: BetaPurpose) {
    guard !busy else { return }
    revision += 1
    self.purpose = purpose
    challenge = nil
    error = nil
    phase = .entry
  }

  func request(email: String, language: String) async {
    guard !busy else { return }
    let cleaned = email.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    guard cleaned.count <= 254, cleaned.contains("@"), !cleaned.contains(where: \.isWhitespace)
    else {
      error = BetaError.invalidEmail.message
      return
    }
    busy = true
    revision += 1
    let operation = revision
    error = nil
    defer { busy = false }
    do {
      let result = try await client.request(email: cleaned, purpose: purpose, language: language)
      guard operation == revision else { return }
      self.email = cleaned
      challenge = result
      resendAt = now().addingTimeInterval(result.resendAfter)
      phase = .code
    } catch {
      guard operation == revision else { return }
      self.error = (error as? BetaError ?? .unavailable).message
    }
  }

  func verify(code: String) async {
    guard !busy, let challenge else { return }
    let cleaned = code.filter { !$0.isWhitespace && $0 != "-" }
    guard cleaned.count == 8, cleaned.allSatisfy({ $0.isASCII && $0.isNumber }) else {
      error = BetaError.invalidCode.message
      return
    }
    busy = true
    error = nil
    let operation = revision
    defer { busy = false }
    do {
      let result = try await client.verify(challenge: challenge.challengeId, code: cleaned)
      guard operation == revision else { return }
      switch result {
      case .authorized(let session):
        guard purpose == .access else { throw BetaError.unavailable }
        try accept(session)
      case .waitlisted:
        guard purpose == .waitlist else { throw BetaError.unavailable }
        self.challenge = nil
        phase = .waitlisted
      }
    } catch {
      guard operation == revision else { return }
      self.error = (error as? BetaError ?? .unavailable).message
    }
  }

  private func accept(_ session: BetaSession) throws {
    guard session.permitsOfflineUse(at: now()), session.token.count == 64, !session.email.isEmpty
    else {
      throw BetaError.unavailable
    }
    try store.save(session)
    self.session = session
    email = session.email
    challenge = nil
    error = nil
    phase = .authorized
  }

  func signOut() {
    revision += 1
    let token = session?.token
    do { try store.delete() } catch { self.error = BetaError.keychain.message }
    session = nil
    challenge = nil
    phase = .entry
    if let token { Task { await client.signOut(token: token) } }
  }
}
