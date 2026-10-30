import Foundation
import Testing

@testable import RoveApp

private final class BetaProtocol: URLProtocol, @unchecked Sendable {
  override class func canInit(with request: URLRequest) -> Bool {
    request.url?.host == "beta.invalid"
  }
  override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
  override func stopLoading() {}
  override func startLoading() {
    let action = request.url!.lastPathComponent
    let status: Int
    let payload: String
    if request.httpMethod != "POST"
      || request.value(forHTTPHeaderField: "Content-Type") != "application/json"
    {
      status = 400
      payload = #"{"error":"invalid_request"}"#
    } else if action == "request" {
      status = 202
      payload = #"{"challengeId":"test-id","expiresAt":1800000600,"resendAfter":60}"#
    } else if action == "verify" {
      status = 200
      payload = #"{"status":"waitlisted"}"#
    } else if action == "session",
      request.value(forHTTPHeaderField: "Authorization") == "Bearer "
        + String(repeating: "a", count: 64)
    {
      status = 200
      payload = """
        {"status":"authorized","token":"\(String(repeating: "a", count: 64))","email":"explorer@example.com",
        "expiresAt":1802592000,"verifiedAt":1800000000,"offlineUntil":1800604800}
        """
    } else {
      status = 401
      payload = #"{"error":"sign_in_required"}"#
    }
    let response = HTTPURLResponse(
      url: request.url!, statusCode: status, httpVersion: "HTTP/1.1",
      headerFields: ["Content-Type": "application/json"])!
    client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
    client?.urlProtocol(self, didLoad: Data(payload.utf8))
    client?.urlProtocolDidFinishLoading(self)
  }
}

struct BetaHTTPTests {
  @Test func serverContractUsesUnixDatesAndBearerAuthorization() async throws {
    let configuration = URLSessionConfiguration.ephemeral
    configuration.protocolClasses = [BetaProtocol.self]
    let transport = URLSession(configuration: configuration)
    defer { transport.invalidateAndCancel() }
    let client = BetaHTTPClient(
      baseURL: URL(string: "https://beta.invalid/")!, transport: transport)
    let challenge = try await client.request(
      email: "explorer@example.com", purpose: .waitlist, language: "en")
    #expect(challenge.expiresAt == Date(timeIntervalSince1970: 1_800_000_600))
    #expect(challenge.resendAfter == 60)
    guard
      case .waitlisted = try await client.verify(challenge: challenge.challengeId, code: "12345678")
    else {
      Issue.record("Confirmed interest must not become an authorized session")
      return
    }
    let session = try await client.refresh(token: String(repeating: "a", count: 64))
    #expect(session.offlineUntil.timeIntervalSince(session.verifiedAt) == 7 * 86400)
    await #expect(throws: BetaError.signInRequired) {
      try await client.refresh(token: String(repeating: "b", count: 64))
    }
  }
}
