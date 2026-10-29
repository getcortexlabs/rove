import SwiftUI

struct ExplorerArtwork: View {
  let strings: AppStrings
  @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
  @Environment(\.visualComfort) private var comfort
  private var reduceMotion: Bool { systemReduceMotion || comfort.reduceMotion }
  @State private var hovering = false
  @State private var greeting = 0
  @State private var bouncing = false

  private let remarks = [
    "A curious little fossil.",
    "One eye on the evidence.",
    "Distant relatives? My specialty.",
    "Very old soul. Very new hobby.",
  ]

  var body: some View {
    VStack(spacing: 0) {
      Button {
        greeting += 1
      } label: {
        ZStack {
          Ellipse()
            .fill(
              RadialGradient(
                colors: [Lab.lilac.opacity(0.12), .clear], center: .center,
                startRadius: 10, endRadius: 140)
            )
            .frame(width: 285, height: 245).offset(y: 18)
          if let image = Lab.explorerImage {
            Image(nsImage: image).resizable().interpolation(.high).scaledToFit()
              .padding(4)
              .shadow(color: .black.opacity(0.2), radius: 12, x: 0, y: 12)
              .rotationEffect(.degrees(reduceMotion ? 0 : (bouncing ? -5 : (hovering ? 2 : 0))))
              .scaleEffect(reduceMotion ? 1 : (bouncing ? 1.04 : (hovering ? 1.015 : 1)))
              .offset(y: bouncing && !reduceMotion ? -7 : 0)
          }
        }.frame(width: 306, height: 300).contentShape(Rectangle())
      }
      .buttonStyle(.plain)
      .onHover { hovering = $0 }
      .animation(reduceMotion ? nil : .spring(response: 0.5, dampingFraction: 0.6), value: hovering)
      .task(id: greeting) {
        guard greeting > 0, !reduceMotion else { return }
        withAnimation(.spring(response: 0.25, dampingFraction: 0.45)) { bouncing = true }
        do { try await Task.sleep(for: .milliseconds(230)) } catch { return }
        withAnimation(.spring(response: 0.45, dampingFraction: 0.5)) { bouncing = false }
      }
      .help(strings("A little hello goes a long way."))
      .accessibilityLabel(strings("Say hello to Rove's fossil explorer"))
      .accessibilityValue(strings(remarks[greeting % remarks.count]))

      HStack(spacing: 7) {
        Image(systemName: "quote.bubble").font(.system(size: 10))
        Text(strings(remarks[greeting % remarks.count])).font(.system(size: 11, design: .rounded))
      }.foregroundStyle(Lab.secondary).padding(.horizontal, 13).padding(.vertical, 8)
        .background(.white.opacity(0.035), in: Capsule())
        .overlay(Capsule().strokeBorder(.white.opacity(0.055), lineWidth: 0.75))
    }.frame(width: 316).accessibilityElement(children: .contain)
  }
}
