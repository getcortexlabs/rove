import Foundation
import Testing

@testable import RoveApp

private final class BenchmarkTransport: URLProtocol, @unchecked Sendable {
  override class func canInit(with request: URLRequest) -> Bool {
    request.url?.host == "benchmark.invalid"
  }
  override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
  override func stopLoading() {}
  override func startLoading() {
    let path = request.url!.lastPathComponent
    let status: Int
    let body: String
    if request.httpMethod != "POST"
      || request.value(forHTTPHeaderField: "Content-Type") != "application/json"
      || request.value(forHTTPHeaderField: "Authorization") != "Bearer test-token"
    {
      status = 401
      body = "{}"
    } else if path == "offline" {
      client?.urlProtocol(self, didFailWithError: URLError(.notConnectedToInternet))
      return
    } else if path == "limited" {
      status = 429
      body = "{}"
    } else {
      status = 201
      body = "{\"id\":\"00000000-0000-4000-8000-000000000039\",\"stored\":true}"
    }
    let response = HTTPURLResponse(
      url: request.url!, statusCode: status, httpVersion: "HTTP/1.1", headerFields: nil)!
    client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
    client?.urlProtocol(self, didLoad: Data(body.utf8))
    client?.urlProtocolDidFinishLoading(self)
  }
}

struct BenchmarkHTTPTests {
  @Test func bearerReceiptAndNetworkFailuresAreChecked() async throws {
    let configuration = URLSessionConfiguration.ephemeral
    configuration.protocolClasses = [BenchmarkTransport.self]
    let transport = URLSession(configuration: configuration)
    defer { transport.invalidateAndCancel() }
    let id = UUID(uuidString: "00000000-0000-4000-8000-000000000039")!
    var client = BenchmarkHTTPClient(
      url: URL(string: "https://benchmark.invalid/result")!, transport: transport)
    try await client.upload(Data("{}".utf8), token: "test-token", id: id)
    await #expect(throws: BenchmarkError.unauthorized) {
      try await client.upload(Data(), token: "wrong", id: id)
    }
    await #expect(throws: BenchmarkError.unavailable) {
      try await client.upload(Data(), token: "test-token", id: UUID())
    }
    client.url = URL(string: "https://benchmark.invalid/limited")!
    await #expect(throws: BenchmarkError.limited) {
      try await client.upload(Data(), token: "test-token", id: id)
    }
    client.url = URL(string: "https://benchmark.invalid/offline")!
    await #expect(throws: URLError.self) {
      try await client.upload(Data(), token: "test-token", id: id)
    }
  }
}
