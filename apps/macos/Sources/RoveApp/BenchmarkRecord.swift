import CryptoKit
import Darwin
import Foundation
import IOKit.ps

struct BenchmarkProtocol: Codable, Equatable, Sendable {
  let id: String
  let inputSHA256: String
  let taxa: Int
  let characters: Int
  let model: String
  let conditioning: String
  let warmupSteps: Int
  let measuredSteps: Int
  let repetitions: Int
  let chains: [Int]
  let seed: UInt64
  let tuning: String
  let timing: String

  static var data: Data {
    get throws {
      guard
        let url = Bundle.module.url(
          forResource: "BenchmarkProtocol", withExtension: "json", subdirectory: "Resources")
      else {
        throw BenchmarkError.invalid
      }
      return try Data(contentsOf: url)
    }
  }
}

struct BenchmarkHardware: Codable, Sendable {
  let model: String
  let chip: String
  let architecture: String
  let physicalCores: Int
  let logicalCores: Int
  let performanceCores: Int?
  let efficiencyCores: Int?
  let memoryBytes: UInt64
  let macOS: String
  let translated: Bool

  static func current() -> Self {
    func text(_ key: String) -> String {
      var size = 0
      guard sysctlbyname(key, nil, &size, nil, 0) == 0, size > 0, size < 512 else {
        return "unknown"
      }
      var bytes = [CChar](repeating: 0, count: size)
      guard sysctlbyname(key, &bytes, &size, nil, 0) == 0 else { return "unknown" }
      return String(decoding: bytes.prefix { $0 != 0 }.map { UInt8(bitPattern: $0) }, as: UTF8.self)
    }
    func integer(_ key: String) -> Int? {
      var value: Int32 = 0
      var size = MemoryLayout<Int32>.size
      return sysctlbyname(key, &value, &size, nil, 0) == 0 ? Int(value) : nil
    }
    let info = ProcessInfo.processInfo
    let os = info.operatingSystemVersion
    return Self(
      model: text("hw.model"), chip: text("machdep.cpu.brand_string"),
      architecture: text("hw.machine"),
      physicalCores: integer("hw.physicalcpu") ?? info.processorCount,
      logicalCores: info.processorCount, performanceCores: integer("hw.perflevel0.physicalcpu"),
      efficiencyCores: integer("hw.perflevel1.physicalcpu"), memoryBytes: info.physicalMemory,
      macOS: "\(os.majorVersion).\(os.minorVersion).\(os.patchVersion)",
      translated: integer("sysctl.proc_translated") == 1)
  }
}

struct BenchmarkConditions: Codable, Sendable {
  let thermal: String
  let lowPower: Bool
  let powerSource: String

  static func current() -> Self {
    let info = ProcessInfo.processInfo
    let thermal: String =
      switch info.thermalState {
      case .nominal: "nominal"
      case .fair: "fair"
      case .serious: "serious"
      case .critical: "critical"
      @unknown default: "unknown"
      }
    var power = "unknown"
    if let snapshot = IOPSCopyPowerSourcesInfo()?.takeRetainedValue(),
      let source = IOPSGetProvidingPowerSourceType(snapshot)?.takeUnretainedValue()
    {
      if source as String == kIOPSACPowerValue { power = "ac" }
      if source as String == kIOPSBatteryPowerValue { power = "battery" }
    }
    return Self(thermal: thermal, lowPower: info.isLowPowerModeEnabled, powerSource: power)
  }
}

struct BenchmarkEngine: Codable, Equatable, Sendable {
  let version: String
  let sha256: String
  let architecture: String
  let os: String
  let optimized: Bool
  let experimentalRepeats: Bool
  let backend: String
}

struct BenchmarkSample: Codable, Equatable, Sendable {
  let chains: Int
  let repetition: Int
  let seconds: Double
  let checksums: [String]
}

struct BenchmarkPayload: Codable, Sendable {
  var schemaVersion = 1
  let id: UUID
  let createdAt: Double
  let appVersion: String
  let appBuild: String
  let appSHA256: String
  let hardware: BenchmarkHardware
  let before: BenchmarkConditions
  var after: BenchmarkConditions?
  var `protocol`: BenchmarkProtocol?
  var protocolSHA256: String?
  var engine: BenchmarkEngine?
  var samples: [BenchmarkSample] = []

  func validate() throws {
    let bytes = try BenchmarkProtocol.data
    let expected = try JSONDecoder().decode(BenchmarkProtocol.self, from: bytes)
    guard schemaVersion == 1, `protocol` == expected, protocolSHA256 == benchmarkHash(bytes),
      after != nil, let engine, engine.optimized, engine.os == "macos",
      benchmarkIsHash(engine.sha256), benchmarkIsHash(appSHA256),
      samples.count == expected.chains.count * expected.repetitions
    else { throw BenchmarkError.invalid }
    for (index, sample) in samples.enumerated() {
      guard sample.chains == expected.chains[index / expected.repetitions],
        sample.repetition == index % expected.repetitions,
        sample.seconds.isFinite, sample.seconds > 0, sample.seconds <= 7200,
        sample.checksums.count == sample.chains, sample.checksums.allSatisfy(benchmarkIsHash)
      else { throw BenchmarkError.invalid }
      let first = samples[index - sample.repetition]
      guard sample.checksums == first.checksums,
        sample.checksums.first == samples.first?.checksums.first
      else {
        throw BenchmarkError.invalid
      }
    }
  }
}

struct BenchmarkRecord: Codable, Identifiable, Sendable {
  var payload: BenchmarkPayload
  var status: String
  var sharedAt: Double?
  var id: UUID { payload.id }

  func submission() throws -> Data {
    guard status == "completed" else { throw BenchmarkError.invalid }
    try payload.validate()
    return try JSONEncoder().encode(payload)
  }
}

enum BenchmarkError: Error, Equatable {
  case invalid, failed, cancelled, unavailable, unauthorized, limited
  var message: String {
    switch self {
    case .invalid: "This benchmark is incomplete or incompatible. Run a new test."
    case .failed: "The benchmark could not finish. Your saved results are still on this Mac."
    case .cancelled: "Benchmark cancelled. Incomplete tests cannot be shared."
    case .unavailable:
      "The result is saved on your Mac. Connect to the internet and try sharing again."
    case .unauthorized: "Please confirm your beta access before sharing this result."
    case .limited: "Too many submissions. Your result is safe here; try sharing later."
    }
  }
}

func benchmarkHash(_ data: Data) -> String {
  SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
}
func benchmarkIsHash(_ value: String) -> Bool {
  value.utf8.count == 64
    && value.utf8.allSatisfy { (48...57).contains($0) || (97...102).contains($0) }
}
func benchmarkFileHash(_ url: URL) throws -> String {
  let handle = try FileHandle(forReadingFrom: url)
  defer { try? handle.close() }
  var hash = SHA256()
  while let data = try handle.read(upToCount: 65536), !data.isEmpty { hash.update(data: data) }
  return hash.finalize().map { String(format: "%02x", $0) }.joined()
}
