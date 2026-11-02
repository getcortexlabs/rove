import CryptoKit
import Foundation
import Testing

@testable import RoveApp

struct UpdaterTests {
  @Test @MainActor func installationWaitsForWorkAndAnotherClick() {
    var busy = true
    var installed = 0
    let gate = UpdateInstallation(busy: { busy })
    #expect(gate.postpone { installed += 1 })
    gate.resume()
    #expect(installed == 0 && gate.isPending && !gate.canResume)
    busy = false
    #expect(installed == 0 && gate.canResume)
    // A new calculation may start while the pending update is visible.
    busy = true
    gate.resume()
    #expect(installed == 0)
    busy = false
    gate.resume()
    gate.resume()
    #expect(installed == 1 && !gate.isPending)
    busy = true
    #expect(gate.postpone { installed += 1 })
    gate.cancel()
    busy = false
    gate.resume()
    #expect(installed == 1)
    #expect(!gate.postpone { installed += 1 })
  }

  @Test @MainActor func updatesRequireAnExplicitDistributionConfiguration() {
    var info: [String: Any] = [
      "RoveUpdatesEnabled": true, "SUFeedURL": "https://rove.getcortexlabs.com/updates/appcast.xml",
      "SUPublicEDKey": Data(repeating: 1, count: 32).base64EncodedString(),
    ]
    #expect(AppUpdater.configured(info))
    info["RoveUpdatesEnabled"] = false
    #expect(!AppUpdater.configured(info))
    info["RoveUpdatesEnabled"] = true
    info["SUFeedURL"] = "http://rove.getcortexlabs.com/updates/appcast.xml"
    #expect(!AppUpdater.configured(info))
    info["SUFeedURL"] = "https://rove.getcortexlabs.com/updates/appcast.xml"
    info["SUPublicEDKey"] = "not-a-key"
    #expect(!AppUpdater.configured(info))
    #expect(!AppUpdater.configured([:]))
  }

  @Test func nestedEngineTasksKeepInstallationBlocked() {
    let activity = EngineActivity()
    activity.begin()
    activity.begin()
    activity.end()
    #expect(activity.isActive)
    activity.end()
    #expect(!activity.isActive)
  }

  @Test func sparkleArchiveSignaturesMatchTheEmbeddedKeyAndRejectTampering() throws {
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-signature-\(UUID())")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let key = Curve25519.Signing.PrivateKey()
    let keyFile = directory.appendingPathComponent("ephemeral.key")
    try key.rawRepresentation.base64EncodedData().write(to: keyFile)
    try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: keyFile.path)
    let archive = directory.appendingPathComponent("update.zip")
    let bytes = Data("signature fixture, not an installable archive".utf8)
    try bytes.write(to: archive)
    let process = Process()
    process.executableURL = root.appendingPathComponent(
      "apps/macos/.build/artifacts/sparkle/Sparkle/bin/sign_update")
    process.arguments = ["--ed-key-file", keyFile.path, "-p", archive.path]
    let output = Pipe()
    process.standardOutput = output
    process.standardError = FileHandle.nullDevice
    try process.run()
    let text = String(decoding: output.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
    process.waitUntilExit()
    #expect(process.terminationStatus == 0)
    let signature = try #require(
      Data(base64Encoded: text.trimmingCharacters(in: .whitespacesAndNewlines)))
    #expect(key.publicKey.isValidSignature(signature, for: bytes))
    #expect(!key.publicKey.isValidSignature(signature, for: bytes + Data([0])))
    #expect(!Curve25519.Signing.PrivateKey().publicKey.isValidSignature(signature, for: bytes))
  }
}
