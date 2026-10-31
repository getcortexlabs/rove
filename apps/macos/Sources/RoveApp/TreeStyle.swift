import AppKit

/// Colors and line metrics shared by the explorer canvas and the exported figure.
struct TreeStyle: Sendable {
  struct Tone: Sendable {
    let hex: UInt32
    var color: NSColor {
      NSColor(
        srgbRed: Double((hex >> 16) & 255) / 255, green: Double((hex >> 8) & 255) / 255,
        blue: Double(hex & 255) / 255, alpha: 1)
    }
    var css: String { String(format: "#%06X", hex) }
  }

  let canvas: Tone
  let ink: Tone
  let branch: Tone
  let highlight: Tone
  let node: Tone
  let constrained: Tone
  let support: Tone
  let hover: Tone
  let scale: Tone
  let branchWidth: Double

  /// The workspace palette: the canvas sits on the same dark surface as the
  /// rest of Rove, with lilac for the selected clade and ice for support.
  static let lab = TreeStyle(
    canvas: Tone(hex: 0x15161E), ink: Tone(hex: 0xF4F2F9), branch: Tone(hex: 0x767A92),
    highlight: Tone(hex: 0xBDB0F7), node: Tone(hex: 0xBDB0F7), constrained: Tone(hex: 0xF0A9BA),
    support: Tone(hex: 0x9ACFE6), hover: Tone(hex: 0x9ACFE6), scale: Tone(hex: 0xB3B3C3),
    branchWidth: 1.5)

  /// A light sheet with ink lines, used for exported figures.
  static let paper = TreeStyle(
    canvas: Tone(hex: 0xF8F7FC), ink: Tone(hex: 0x303141), branch: Tone(hex: 0x75798D),
    highlight: Tone(hex: 0x8064C1), node: Tone(hex: 0x8064C1), constrained: Tone(hex: 0xB65C7C),
    support: Tone(hex: 0x4C6F9C), hover: Tone(hex: 0x3C7FA8), scale: Tone(hex: 0x303141),
    branchWidth: 1.4)
}
