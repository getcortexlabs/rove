import SwiftUI

struct BranchArtwork: View {
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @State private var booped = false
  @State private var hovering = false
  @State private var greeting = false

  var body: some View {
    ZStack {
      Canvas { context, size in
        for x in stride(from: 12.0, to: size.width, by: 24) {
          for y in stride(from: 12.0, to: size.height, by: 24) {
            context.fill(
              Path(ellipseIn: CGRect(x: x, y: y, width: 1.4, height: 1.4)),
              with: .color(Lab.muted.opacity(0.2)))
          }
        }
      }
      Ellipse().stroke(Lab.border, style: StrokeStyle(lineWidth: 1, dash: [3, 7]))
        .frame(width: 290, height: 205).rotationEffect(.degrees(-25))
      branches
      Circle().fill(Lab.lilac).frame(width: 48, height: 48).position(x: 77, y: 78)
        .overlay(alignment: .topLeading) {
          Image(systemName: "sparkle").font(.system(size: 23, weight: .light))
            .foregroundStyle(Lab.lilac).offset(x: 97, y: 13)
        }
      Circle().fill(Lab.coral).frame(width: 31, height: 31).position(x: 289, y: 72)
      Circle().fill(Lab.lime.opacity(0.85)).frame(width: 23, height: 23).position(x: 298, y: 212)
      Circle().strokeBorder(Lab.lilac.opacity(0.8), lineWidth: 3).frame(width: 18, height: 18)
        .position(x: 70, y: 240)
      Button {
        greeting.toggle()
        guard !reduceMotion else { return }
        withAnimation(.spring(response: 0.22, dampingFraction: 0.35)) { booped = true }
        Task { @MainActor in
          try? await Task.sleep(for: .milliseconds(250))
          withAnimation(.spring(response: 0.45, dampingFraction: 0.5)) { booped = false }
        }
      } label: {
        ZStack {
          RoundedRectangle(cornerRadius: 43).fill(Lab.lime.gradient)
            .rotationEffect(.degrees(-12))
            .shadow(color: Lab.lime.opacity(0.1), radius: 30, y: 12)
          HStack(spacing: 12) {
            eye
            eye
          }.offset(y: -10)
          Path { path in
            path.move(to: CGPoint(x: 57, y: 87))
            path.addQuadCurve(
              to: CGPoint(x: 87, y: 86), control: CGPoint(x: 73, y: greeting ? 112 : 102))
          }.stroke(Lab.background, style: StrokeStyle(lineWidth: 4, lineCap: .round))
        }.frame(width: 142, height: 132)
      }
      .buttonStyle(.plain)
      .scaleEffect(booped ? 0.88 : (hovering && !reduceMotion ? 1.035 : 1))
      .rotationEffect(.degrees(booped ? 8 : 0))
      .onHover { hovering = $0 }
      .animation(reduceMotion ? nil : .spring(response: 0.4, dampingFraction: 0.6), value: hovering)
      .position(x: 185, y: 151)
      .help("A little curiosity goes a long way.")
      .accessibilityLabel("Say hello to Rove's curious little branch")
      Text(greeting ? "hello, fellow explorer." : "curious by nature.")
        .font(.system(size: 10, weight: .medium, design: .monospaced))
        .foregroundStyle(Lab.secondary).rotationEffect(.degrees(-6)).position(x: 221, y: 267)
      Image(systemName: "sparkle").font(.system(size: 16, weight: .light))
        .foregroundStyle(Lab.lime).position(x: 323, y: 135)
    }
    .frame(width: 360, height: 292)
    .accessibilityElement(children: .contain)
  }

  private var eye: some View {
    ZStack {
      Capsule().fill(Color(hex: 0xFAFFE9)).frame(width: 30, height: 39)
      Capsule().fill(Lab.background).frame(width: 12, height: 21)
        .offset(x: hovering ? 3 : 1, y: 4)
    }
  }

  private var branches: some View {
    Canvas { context, _ in
      let curves: [(CGPoint, CGPoint, CGPoint, Color)] = [
        (CGPoint(x: 162, y: 139), CGPoint(x: 111, y: 139), CGPoint(x: 77, y: 78), Lab.lilac),
        (CGPoint(x: 207, y: 136), CGPoint(x: 264, y: 132), CGPoint(x: 289, y: 72), Lab.coral),
        (CGPoint(x: 204, y: 177), CGPoint(x: 254, y: 215), CGPoint(x: 298, y: 212), Lab.lime),
        (CGPoint(x: 164, y: 185), CGPoint(x: 125, y: 248), CGPoint(x: 70, y: 240), Lab.lilac),
      ]
      for (start, control, end, color) in curves {
        var path = Path()
        path.move(to: start)
        path.addQuadCurve(to: end, control: control)
        context.stroke(
          path, with: .color(color.opacity(0.85)),
          style: StrokeStyle(lineWidth: 8, lineCap: .round))
      }
    }
  }
}
