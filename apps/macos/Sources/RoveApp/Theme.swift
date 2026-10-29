import AppKit
import CoreText
import SwiftUI

enum Lab {
  static let background = Color(hex: 0x11130F)
  static let sidebar = Color(hex: 0x151812)
  static let surface = Color(hex: 0x1A1E17)
  static let raised = Color(hex: 0x22271D)
  static let border = Color(hex: 0x343B2B)
  static let ink = Color(hex: 0xF0F0E7)
  static let secondary = Color(hex: 0xADB4A2)
  static let muted = Color(hex: 0x858F78)
  static let lime = Color(hex: 0xC4F563)
  static let lilac = Color(hex: 0xB7A2FA)
  static let coral = Color(hex: 0xFFA3BD)

  static func resource(_ name: String, extension ext: String) -> URL? {
    // SwiftPM's generated search paths vary between its native and Xcode build systems.
    if let url = Bundle.main.resourceURL?.appendingPathComponent("Rove_RoveApp.bundle"),
      let bundled = Bundle(url: url)
    {
      return bundled.url(forResource: name, withExtension: ext, subdirectory: "Resources")
    }
    return Bundle.module.url(forResource: name, withExtension: ext, subdirectory: "Resources")
  }

  static func registerFonts() {
    guard let url = resource("BricolageGrotesque", extension: "ttf") else { return }
    CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
  }

  static func display(_ size: CGFloat, weight: Font.Weight = .bold) -> Font {
    .custom("BricolageGrotesque-Regular", size: size).weight(weight)
  }
}

extension Color {
  init(hex: UInt32) {
    self.init(
      .sRGB, red: Double((hex >> 16) & 255) / 255,
      green: Double((hex >> 8) & 255) / 255, blue: Double(hex & 255) / 255, opacity: 1)
  }
}

struct Eyebrow: View {
  let text: String
  var color: Color = Lab.muted
  var body: some View {
    Text(text.uppercased()).font(.system(size: 10, weight: .medium, design: .monospaced))
      .tracking(1.5).foregroundStyle(color)
  }
}

struct LabButtonStyle: ButtonStyle {
  var prominent = false
  var compact = false
  @Environment(\.isEnabled) private var enabled
  @Environment(\.accessibilityReduceMotion) private var reduceMotion

  func makeBody(configuration: Configuration) -> some View {
    configuration.label
      .font(.system(size: compact ? 12 : 13, weight: .semibold))
      .padding(.horizontal, compact ? 12 : 18).padding(.vertical, compact ? 9 : 13)
      .foregroundStyle(prominent ? Lab.background : Lab.ink)
      .background(prominent ? Lab.lime : Lab.raised, in: RoundedRectangle(cornerRadius: 11))
      .overlay(RoundedRectangle(cornerRadius: 11).strokeBorder(prominent ? .clear : Lab.border))
      .scaleEffect(configuration.isPressed && !reduceMotion ? 0.97 : 1)
      .opacity(enabled ? (configuration.isPressed ? 0.85 : 1) : 0.4)
      .animation(reduceMotion ? nil : .easeOut(duration: 0.15), value: configuration.isPressed)
      .contentShape(RoundedRectangle(cornerRadius: 11))
  }
}

struct LabPanel<Content: View>: View {
  @ViewBuilder let content: Content
  var body: some View {
    content.padding(22).background(Lab.surface, in: RoundedRectangle(cornerRadius: 18))
      .overlay(RoundedRectangle(cornerRadius: 18).strokeBorder(Lab.border.opacity(0.7)))
  }
}
