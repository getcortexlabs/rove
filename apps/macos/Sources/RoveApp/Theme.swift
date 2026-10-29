import AppKit
import CoreText
import SwiftUI

struct VisualComfort: Equatable {
  var reduceMotion = false
  var reduceTransparency = false
}

private struct VisualComfortKey: EnvironmentKey {
  static let defaultValue = VisualComfort()
}

extension EnvironmentValues {
  var visualComfort: VisualComfort {
    get { self[VisualComfortKey.self] }
    set { self[VisualComfortKey.self] = newValue }
  }
}

enum Lab {
  static let background = Color(hex: 0x101117)
  static let sidebar = Color(hex: 0x23242F)
  static let surface = Color(hex: 0x1B1C25)
  static let raised = Color(hex: 0x272934)
  static let border = Color(hex: 0x373946)
  static let ink = Color(hex: 0xF4F2F9)
  static let secondary = Color(hex: 0xB3B3C3)
  static let muted = Color(hex: 0x8D8FA4)
  static let accent = Color(hex: 0xBDB0F7)
  static let lilac = Color(hex: 0xBDB0F7)
  static let ice = Color(hex: 0x9ACFE6)
  static let coral = Color(hex: 0xF0A9BA)

  @MainActor static let explorerImage: NSImage? = {
    resource("RoveExplorer", extension: "png").flatMap(NSImage.init(contentsOf:))
  }()

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
  @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
  @Environment(\.visualComfort) private var comfort
  private var reduceMotion: Bool { systemReduceMotion || comfort.reduceMotion }
  @State private var hovering = false

  func makeBody(configuration: Configuration) -> some View {
    configuration.label
      .font(.system(size: compact ? 12 : 13, weight: .semibold))
      .padding(.horizontal, compact ? 12 : 18).padding(.vertical, compact ? 9 : 13)
      .foregroundStyle(prominent ? Lab.background : Lab.ink)
      .background(
        LinearGradient(
          colors: prominent
            ? [Color(hex: 0xD4C9FF), Lab.accent]
            : [Lab.raised.opacity(hovering ? 1 : 0.8), Lab.surface],
          startPoint: .topLeading, endPoint: .bottomTrailing),
        in: RoundedRectangle(cornerRadius: 12)
      )
      .overlay(
        RoundedRectangle(cornerRadius: 12).strokeBorder(
          .white.opacity(prominent ? 0.24 : (hovering ? 0.2 : 0.09)), lineWidth: 0.75)
      )
      .scaleEffect(configuration.isPressed && !reduceMotion ? 0.97 : 1)
      .opacity(enabled ? (configuration.isPressed ? 0.85 : 1) : 0.4)
      .animation(reduceMotion ? nil : .easeOut(duration: 0.15), value: configuration.isPressed)
      .onHover { hovering = $0 }
      .contentShape(RoundedRectangle(cornerRadius: 11))
  }
}

struct WorkspaceBackdrop: View {
  var body: some View {
    ZStack {
      Lab.background
      RadialGradient(
        colors: [Lab.lilac.opacity(0.13), .clear], center: .topLeading,
        startRadius: 0, endRadius: 620)
      RadialGradient(
        colors: [Lab.ice.opacity(0.055), .clear], center: .bottomLeading,
        startRadius: 0, endRadius: 500)
    }.ignoresSafeArea().allowsHitTesting(false).accessibilityHidden(true)
  }
}

struct SidebarGlass: View {
  @Environment(\.accessibilityReduceTransparency) private var systemReduceTransparency
  @Environment(\.visualComfort) private var comfort
  private var reduceTransparency: Bool { systemReduceTransparency || comfort.reduceTransparency }
  @Environment(\.colorSchemeContrast) private var contrast
  private let shape = RoundedRectangle(cornerRadius: 22, style: .continuous)

  var body: some View {
    Group {
      if reduceTransparency || contrast == .increased {
        shape.fill(Lab.sidebar)
          .overlay(shape.strokeBorder(.white.opacity(0.24), lineWidth: 0.75))
      } else {
        glass
          .background(
            shape.fill(
              LinearGradient(
                colors: [.white.opacity(0.065), .white.opacity(0.025)],
                startPoint: .topLeading, endPoint: .bottomTrailing))
          )
          .overlay(
            shape.strokeBorder(
              LinearGradient(
                colors: [.white.opacity(0.19), .white.opacity(0.035), .white.opacity(0.09)],
                startPoint: .topLeading, endPoint: .bottomTrailing), lineWidth: 0.75))
      }
    }.allowsHitTesting(false)
  }

  @ViewBuilder private var glass: some View {
    #if compiler(>=6.2)
      if #available(macOS 26, *) {
        shape.fill(.clear).glassEffect(.regular, in: shape)
      } else {
        frosted
      }
    #else
      frosted
    #endif
  }

  private var frosted: some View {
    shape.fill(.ultraThinMaterial)
      .overlay(
        shape.strokeBorder(
          LinearGradient(
            colors: [.white.opacity(0.22), .white.opacity(0.04), .white.opacity(0.1)],
            startPoint: .topLeading, endPoint: .bottomTrailing), lineWidth: 0.75))
  }
}

struct LabPanel<Content: View>: View {
  @ViewBuilder let content: Content
  var body: some View {
    content.padding(22).background(Lab.surface, in: RoundedRectangle(cornerRadius: 18))
      .overlay(RoundedRectangle(cornerRadius: 18).strokeBorder(Lab.border.opacity(0.7)))
  }
}
