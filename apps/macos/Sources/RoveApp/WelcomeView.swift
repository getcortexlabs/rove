import SwiftUI

struct WelcomeView: View {
  @Bindable var workspace: Workspace
  @State private var targeted = false
  @State private var hovering = false
  @Environment(\.accessibilityReduceMotion) private var reduceMotion

  var body: some View {
    VStack(alignment: .leading, spacing: 28) {
      HStack(spacing: 0) {
        VStack(alignment: .leading, spacing: 18) {
          HStack(spacing: 8) {
            Circle().fill(Lab.lime).frame(width: 6, height: 6)
            Eyebrow(text: "A little curiosity goes a long way", color: Lab.secondary)
          }
          VStack(alignment: .leading, spacing: -3) {
            Text("Small clues.").foregroundStyle(Lab.ink)
            Text("Big discoveries.").foregroundStyle(Lab.lime)
          }.font(Lab.display(48)).tracking(-1.8).fixedSize(horizontal: true, vertical: false)
          Text("Every character has a story.\nLet's find out how they connect.")
            .font(.system(size: 15)).foregroundStyle(Lab.secondary).lineSpacing(5)
        }.frame(maxWidth: .infinity, alignment: .leading)
        BranchArtwork().frame(width: 335).padding(.trailing, -10)
      }.padding(.top, 6)

      Button(action: workspace.chooseFile) {
        VStack(spacing: 15) {
          ZStack(alignment: .bottomTrailing) {
            RoundedRectangle(cornerRadius: 14).fill(Lab.lime.opacity(0.09))
              .frame(width: 58, height: 62).rotationEffect(.degrees(targeted || hovering ? 0 : -7))
            Image(systemName: "square.grid.3x3.fill")
              .font(.system(size: 26, weight: .light)).foregroundStyle(Lab.lime)
              .frame(width: 58, height: 62)
            Image(systemName: "plus.circle.fill").font(.system(size: 18))
              .foregroundStyle(Lab.lime, Lab.surface).offset(x: 5, y: 2)
          }
          VStack(spacing: 7) {
            Text(targeted ? "That's the one. Drop it here." : "Your next discovery starts here.")
              .font(Lab.display(21, weight: .semibold)).foregroundStyle(Lab.ink)
            Text("Drop a NEXUS matrix, or click to browse")
              .font(.system(size: 13)).foregroundStyle(Lab.secondary)
          }
          HStack(spacing: 9) {
            Text(".nex  ·  .nexus").font(.system(size: 10, design: .monospaced))
            Circle().fill(Lab.border).frame(width: 3, height: 3)
            Text("Morphological characters").font(.system(size: 10))
          }.foregroundStyle(Lab.muted).padding(.top, 2)
        }
        .frame(maxWidth: .infinity).padding(.vertical, 31)
        .background(
          (targeted ? Lab.lime.opacity(0.075) : Lab.surface), in: RoundedRectangle(cornerRadius: 20)
        )
        .overlay(
          RoundedRectangle(cornerRadius: 20).strokeBorder(
            targeted || hovering ? Lab.lime.opacity(0.65) : Lab.border,
            style: StrokeStyle(lineWidth: 1, dash: [5, 5])))
      }
      .buttonStyle(.plain)
      .onHover { hovering = $0 }
      .animation(reduceMotion ? nil : .easeOut(duration: 0.2), value: hovering)
      .dropDestination(for: URL.self) { urls, _ in
        guard urls.count == 1, let url = urls.first else {
          workspace.error = "Drop one matrix at a time."
          return false
        }
        workspace.load(url)
        return true
      } isTargeted: {
        targeted = $0
      }
      .accessibilityLabel("Open or drop a morphological NEXUS matrix")

      HStack(spacing: 20) {
        HStack(spacing: 12) {
          Image(systemName: "sparkles").font(.system(size: 21)).foregroundStyle(Lab.lilac)
          VStack(alignment: .leading, spacing: 5) {
            Text("Just looking around?").font(.system(size: 13, weight: .semibold))
            Text("Meet a tiny matrix with a lot of family history.")
              .font(.system(size: 11)).foregroundStyle(Lab.secondary)
          }
        }
        Spacer(minLength: 10)
        Button(action: workspace.loadExample) {
          HStack(spacing: 9) {
            Text("Try an example")
            Image(systemName: "arrow.up.right")
          }
        }.buttonStyle(LabButtonStyle(compact: true))
      }.padding(20)
        .background(Lab.lilac.opacity(0.045), in: RoundedRectangle(cornerRadius: 16))
        .overlay(RoundedRectangle(cornerRadius: 16).strokeBorder(Lab.lilac.opacity(0.15)))

      HStack(spacing: 8) {
        Image(systemName: "lock.shield").font(.system(size: 11))
        Text("Your matrix stays on your Mac. Always your work.").font(.system(size: 11))
        Spacer()
        Text("LET CURIOSITY BRANCH OUT ↗").font(.system(size: 9, design: .monospaced)).tracking(1)
      }.foregroundStyle(Lab.muted).padding(.bottom, 8)
    }
  }
}
