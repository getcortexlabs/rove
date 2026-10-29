import SwiftUI

struct WelcomeView: View {
  @Bindable var workspace: Workspace
  private var strings: AppStrings { workspace.strings }
  @State private var targeted = false
  @State private var hovering = false
  @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
  @Environment(\.visualComfort) private var comfort
  private var reduceMotion: Bool { systemReduceMotion || comfort.reduceMotion }

  var body: some View {
    VStack(alignment: .leading, spacing: 24) {
      HStack(spacing: 12) {
        VStack(alignment: .leading, spacing: 22) {
          HStack(spacing: 7) {
            Image(systemName: "sparkle").foregroundStyle(Lab.ice)
            Text(strings("GOOD SCIENCE STARTS WITH CURIOSITY"))
              .font(.system(size: 9, weight: .medium, design: .monospaced)).tracking(1.1)
              .foregroundStyle(Lab.secondary)
          }
          ViewThatFits(in: .horizontal) {
            headline(size: 46)
            headline(size: 38)
          }
          Text(
            strings(
              "A new perspective on life's little details.\nBring a matrix. Follow the connections."
            )
          )
          .font(.system(size: 14)).foregroundStyle(Lab.secondary).lineSpacing(6)
          HStack(spacing: 6) {
            Image(systemName: "point.3.connected.trianglepath.dotted").foregroundStyle(Lab.accent)
            Text(strings("Your next family tree starts here.")).foregroundStyle(Lab.muted)
          }.font(.system(size: 11)).padding(.top, 1)
        }.frame(maxWidth: .infinity, alignment: .leading)
        ExplorerArtwork(strings: strings)
      }.padding(.top, 1).padding(.bottom, 5)

      importCard
      exampleCard

      HStack(spacing: 7) {
        Image(systemName: "lock.shield").font(.system(size: 10)).foregroundStyle(Lab.ice)
        Text(strings("Your matrix stays on your Mac.")).font(.system(size: 11))
        Spacer()
        Text(strings("Made for curious minds.")).font(.system(size: 10)).foregroundStyle(Lab.muted)
      }.foregroundStyle(Lab.secondary).padding(.horizontal, 3).padding(.top, 2)
    }
  }

  private var importCard: some View {
    Button(action: workspace.chooseFile) {
      HStack(spacing: 23) {
        MatrixDocumentIcon(highlighted: targeted || hovering)
        VStack(alignment: .leading, spacing: 9) {
          Text(strings(targeted ? "That's the one. Drop it here." : "Start with your matrix"))
            .font(Lab.display(23, weight: .semibold)).tracking(-0.4).foregroundStyle(Lab.ink)
          Text(strings("Drop a NEXUS file here, or browse your Mac."))
            .font(.system(size: 12)).foregroundStyle(Lab.secondary)
          HStack(spacing: 8) {
            Text(".nex").font(.system(size: 10, design: .monospaced))
            Text(".nexus").font(.system(size: 10, design: .monospaced))
            Text(strings("·  Morphological characters")).font(.system(size: 10))
          }.foregroundStyle(Lab.muted).padding(.top, 7)
        }
        Spacer(minLength: 4)
        Image(systemName: "arrow.up.right")
          .font(.system(size: 16, weight: .medium)).foregroundStyle(Lab.accent)
          .frame(width: 44, height: 44)
          .background(Lab.accent.opacity(targeted || hovering ? 0.18 : 0.09), in: Circle())
          .overlay(Circle().strokeBorder(Lab.accent.opacity(0.14), lineWidth: 0.75))
      }
      .padding(.horizontal, 28).padding(.vertical, 30)
      .frame(maxWidth: .infinity, minHeight: 170, alignment: .leading)
      .background(
        LinearGradient(
          colors: [Color(hex: 0x252332), Color(hex: 0x1B1D28)],
          startPoint: .topLeading, endPoint: .bottomTrailing),
        in: RoundedRectangle(cornerRadius: 22)
      )
      .overlay(
        RoundedRectangle(cornerRadius: 22).strokeBorder(
          LinearGradient(
            colors: [Lab.accent.opacity(targeted || hovering ? 0.7 : 0.3), Lab.ice.opacity(0.12)],
            startPoint: .topLeading, endPoint: .bottomTrailing),
          lineWidth: targeted ? 1.5 : 0.75))
    }
    .buttonStyle(.plain)
    .onHover { hovering = $0 }
    .animation(reduceMotion ? nil : .easeOut(duration: 0.2), value: hovering)
    .animation(reduceMotion ? nil : .easeOut(duration: 0.2), value: targeted)
    .dropDestination(for: URL.self) { urls, _ in
      guard urls.count == 1, let url = urls.first else {
        workspace.errorDetails = nil
        workspace.error = "Drop one matrix at a time."
        return false
      }
      workspace.load(url)
      return true
    } isTargeted: {
      targeted = $0
    }
    .accessibilityLabel(strings("Open or drop a morphological NEXUS matrix"))
  }

  private func headline(size: CGFloat) -> some View {
    VStack(alignment: .leading, spacing: 0) {
      Text(strings("Small clues.")).foregroundStyle(Lab.ink)
      Text(strings("Big discoveries."))
        .foregroundStyle(
          LinearGradient(
            colors: [Color(hex: 0xDDD5FF), Lab.accent, Lab.ice],
            startPoint: .leading, endPoint: .trailing))
    }.font(Lab.display(size)).tracking(-1.7).fixedSize(horizontal: true, vertical: false)
  }

  private var exampleCard: some View {
    HStack(spacing: 14) {
      Image(systemName: "sparkles").font(.system(size: 18, weight: .light))
        .foregroundStyle(Lab.ice).frame(width: 37, height: 37)
        .background(Lab.ice.opacity(0.06), in: RoundedRectangle(cornerRadius: 11))
      VStack(alignment: .leading, spacing: 5) {
        Text(strings("Just a little curious?")).font(.system(size: 13, weight: .semibold))
        Text(strings("Take our tiny example matrix for a spin.")).font(.system(size: 11))
          .foregroundStyle(Lab.secondary)
      }
      Spacer(minLength: 10)
      Button(action: workspace.loadExample) {
        HStack(spacing: 8) {
          Text(strings("Try an example"))
          Image(systemName: "arrow.right").font(.system(size: 10))
        }
      }.buttonStyle(LabButtonStyle(compact: true))
    }.padding(19)
      .background(.white.opacity(0.025), in: RoundedRectangle(cornerRadius: 17))
      .overlay(
        RoundedRectangle(cornerRadius: 17).strokeBorder(.white.opacity(0.07), lineWidth: 0.75))
  }
}

private struct MatrixDocumentIcon: View {
  let highlighted: Bool
  @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
  @Environment(\.visualComfort) private var comfort
  private var reduceMotion: Bool { systemReduceMotion || comfort.reduceMotion }

  var body: some View {
    ZStack {
      RoundedRectangle(cornerRadius: 12).fill(Lab.ice.opacity(0.09))
        .frame(width: 59, height: 74).rotationEffect(.degrees(-14)).offset(x: -6, y: 1)
      VStack(spacing: 8) {
        HStack(spacing: 3) {
          Circle().fill(Lab.ice).frame(width: 4, height: 4)
          RoundedRectangle(cornerRadius: 2).fill(Lab.secondary.opacity(0.3)).frame(
            width: 23, height: 3)
        }.frame(maxWidth: .infinity, alignment: .leading)
        Grid(horizontalSpacing: 4, verticalSpacing: 4) {
          ForEach(0..<3) { row in
            GridRow {
              ForEach(0..<4) { column in
                RoundedRectangle(cornerRadius: 2)
                  .fill((row + column).isMultiple(of: 3) ? Lab.ice : Lab.accent.opacity(0.35))
                  .frame(width: 6, height: 6)
              }
            }
          }
        }
      }.padding(13).background(
        LinearGradient(
          colors: [Color(hex: 0x424052), Color(hex: 0x2B293A)],
          startPoint: .topLeading, endPoint: .bottomTrailing),
        in: RoundedRectangle(cornerRadius: 12)
      )
      .overlay(
        RoundedRectangle(cornerRadius: 12).strokeBorder(.white.opacity(0.14), lineWidth: 0.75)
      )
      .rotationEffect(.degrees(highlighted && !reduceMotion ? 3 : -3))
      .shadow(color: .black.opacity(0.2), radius: 6, y: 5)
    }.frame(width: 79, height: 88).accessibilityHidden(true)
  }
}
