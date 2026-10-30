import AppKit
import Observation

protocol BenchmarkSharing: Sendable {
  func upload(_ data: Data, token: String, id: UUID) async throws
}

struct BenchmarkHTTPClient: BenchmarkSharing {
  var url = URL(string: "https://getcortexlabs.com/api/rove/benchmarks")!
  var transport: URLSession = {
    let config = URLSessionConfiguration.ephemeral
    config.timeoutIntervalForRequest = 20
    config.httpShouldSetCookies = false
    config.urlCache = nil
    return URLSession(configuration: config)
  }()

  func upload(_ data: Data, token: String, id: UUID) async throws {
    guard data.count <= 16384 else { throw BenchmarkError.invalid }
    var request = URLRequest(url: url)
    request.httpMethod = "POST"
    request.setValue("application/json", forHTTPHeaderField: "Content-Type")
    request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
    request.httpBody = data
    let (reply, response) = try await transport.data(for: request)
    guard let response = response as? HTTPURLResponse, reply.count < 4096 else {
      throw BenchmarkError.unavailable
    }
    switch response.statusCode {
    case 200, 201: break
    case 401, 403: throw BenchmarkError.unauthorized
    case 429: throw BenchmarkError.limited
    default: throw BenchmarkError.unavailable
    }
    struct Receipt: Decodable {
      let id: UUID
      let stored: Bool
    }
    guard let receipt = try? JSONDecoder().decode(Receipt.self, from: reply), receipt.id == id,
      receipt.stored
    else {
      throw BenchmarkError.unavailable
    }
  }
}

struct BenchmarkStore: Sendable {
  var directory: URL = FileManager.default.urls(
    for: .applicationSupportDirectory, in: .userDomainMask)[0]
    .appendingPathComponent("Rove/Benchmarks", isDirectory: true)

  func save(_ record: BenchmarkRecord) throws {
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    try JSONEncoder().encode(record).write(
      to: directory.appendingPathComponent("\(record.id).json"), options: .atomic)
  }

  func load() throws -> [BenchmarkRecord] {
    guard FileManager.default.fileExists(atPath: directory.path) else { return [] }
    let files = try FileManager.default.contentsOfDirectory(
      at: directory, includingPropertiesForKeys: [.fileSizeKey, .isRegularFileKey])
    var records: [BenchmarkRecord] = []
    for file in files where file.pathExtension == "json" {
      let info = try file.resourceValues(forKeys: [.fileSizeKey, .isRegularFileKey])
      guard info.isRegularFile == true, (info.fileSize ?? 0) <= 32768,
        var record = try? JSONDecoder().decode(BenchmarkRecord.self, from: Data(contentsOf: file)),
        file.deletingPathExtension().lastPathComponent == record.id.uuidString
      else { continue }
      if record.status == "running" {
        record.status = "interrupted"
        try save(record)
      }
      records.append(record)
    }
    return records.sorted { $0.payload.createdAt > $1.payload.createdAt }
  }
}

@MainActor @Observable final class Benchmark {
  private(set) var records: [BenchmarkRecord] = []
  var selectedID: UUID?
  private(set) var isRunning = false
  private(set) var sharing = false
  private(set) var completedMeasurements = 0
  private(set) var currentChains = 1
  private(set) var error: String?
  private var worker: BenchmarkProcess?
  private var task: Task<Void, Never>?
  let store: BenchmarkStore
  private let client: any BenchmarkSharing

  init(
    store: BenchmarkStore = BenchmarkStore(), client: any BenchmarkSharing = BenchmarkHTTPClient()
  ) {
    self.store = store
    self.client = client
    do { records = try store.load() } catch { self.error = "Saved benchmarks could not be opened." }
    selectedID = records.first?.id
  }

  var selected: BenchmarkRecord? { records.first { $0.id == selectedID } }

  func start(executable: URL) {
    guard !isRunning, !sharing else { return }
    isRunning = true
    error = nil
    completedMeasurements = 0
    currentChains = 1
    let worker = BenchmarkProcess()
    self.worker = worker
    let id = UUID()
    selectedID = id
    task = Task { [self] in
      var record: BenchmarkRecord?
      let activity = ProcessInfo.processInfo.beginActivity(
        options: [.userInitiated, .idleSystemSleepDisabled], reason: "Rove benchmark")
      defer {
        ProcessInfo.processInfo.endActivity(activity)
        isRunning = false
        self.worker = nil
      }
      do {
        let appURL = Bundle.main.executableURL ?? executable
        let appHash = try await Task.detached { try benchmarkFileHash(appURL) }.value
        let payload = BenchmarkPayload(
          id: id, createdAt: Date().timeIntervalSince1970,
          appVersion: Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString")
            as? String ?? "development",
          appBuild: Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String
            ?? "development",
          appSHA256: appHash, hardware: .current(), before: .current())
        var current = BenchmarkRecord(payload: payload, status: "running")
        record = current
        try keep(current)
        let events = try await worker.execute(executable: executable) { event in
          Task { @MainActor [weak self] in
            guard let self, self.isRunning, self.selectedID == id else { return }
            if event.event == "warming" { self.currentChains = event.chains ?? 1 }
            if event.event == "measurement" { self.completedMeasurements += 1 }
          }
        }
        for event in events {
          if event.event == "started" {
            current.payload.protocol = event.protocol
            current.payload.protocolSHA256 = event.protocolSHA256
            current.payload.engine = event.engine
          }
          if let sample = event.sample { current.payload.samples.append(sample) }
        }
        current.payload.after = .current()
        record = current
        try current.payload.validate()
        current.status = "completed"
        try keep(current)
        completedMeasurements = 6
      } catch {
        if var record {
          record.status = (error as? BenchmarkError) == .cancelled ? "cancelled" : "failed"
          record.payload.after = .current()
          do { try keep(record) } catch { self.error = "The benchmark record could not be saved." }
        }
        if self.error == nil {
          self.error = (error as? BenchmarkError)?.message ?? BenchmarkError.failed.message
        }
      }
    }
  }

  private func keep(_ record: BenchmarkRecord) throws {
    try store.save(record)
    records.removeAll { $0.id == record.id }
    records.insert(record, at: 0)
  }

  func cancel() { worker?.cancel() }
  func waitForExit() async { await task?.value }

  func share(token: String?) async {
    guard !isRunning, !sharing, var record = selected, record.sharedAt == nil else { return }
    guard let token else {
      error = BenchmarkError.unauthorized.message
      return
    }
    sharing = true
    error = nil
    defer { sharing = false }
    do {
      let bytes = try record.submission()
      // The original result is durable before any network request.
      try store.save(record)
      try await client.upload(bytes, token: token, id: record.id)
      record.sharedAt = Date().timeIntervalSince1970
      try keep(record)
    } catch {
      self.error = (error as? BenchmarkError)?.message ?? BenchmarkError.unavailable.message
    }
  }
}
