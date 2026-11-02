import CryptoKit
import Foundation

// Only public material is passed as arguments; signing stays in Sparkle's tool.
guard CommandLine.arguments.count == 4,
  let key = Data(base64Encoded: CommandLine.arguments[1]),
  let signature = Data(base64Encoded: CommandLine.arguments[3])
else { fatalError("usage: verify-update public-key archive signature") }
let publicKey = try Curve25519.Signing.PublicKey(rawRepresentation: key)
let archive = try Data(
  contentsOf: URL(fileURLWithPath: CommandLine.arguments[2]), options: .mappedIfSafe)
guard publicKey.isValidSignature(signature, for: archive) else {
  fputs("Update signature does not match the public key in Rove.app\n", stderr)
  exit(1)
}
