import SwiftUI

struct AboutView: View {
  @Bindable var preferences: AppPreferences
  var version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
  @Environment(\.dismissWindow) private var dismissWindow
  private var strings: AppStrings { preferences.strings }

  var body: some View {
    VStack(alignment: .leading, spacing: 23) {
      identity

      Rectangle()
        .fill(
          LinearGradient(
            colors: [Lab.accent.opacity(0.45), Lab.ice.opacity(0.15), .clear],
            startPoint: .leading, endPoint: .trailing)
        ).frame(height: 1).accessibilityHidden(true)

      VStack(alignment: .leading, spacing: 8) {
        Text(strings("We just wanted a shorter wait."))
          .font(Lab.display(20, weight: .semibold))
        Text(
          strings(
            "BEAST analyses were taking a while on a Mac M1. Rafael wanted to see if he could build something faster, and Thiago joined him with his experience in biology and research. The idea was to save some time. So, naturally, they started a whole new project."
          )
        )
        .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
        .fixedSize(horizontal: false, vertical: true)
      }

      VStack(spacing: 17) {
        credit("Rafael Augusto", initials: "RA", role: "Software & design", color: Lab.accent)
        Rectangle().fill(Lab.border.opacity(0.6)).frame(height: 1)
          .padding(.leading, 54).accessibilityHidden(true)
        credit("Thiago J. Silva", initials: "TS", role: "Biology & research", color: Lab.ice)
      }.padding(20).background { SidebarGlass() }

      HStack(spacing: 10) {
        externalLink("Website", url: URL(string: "https://rove.getcortexlabs.com/")!)
        externalLink("Field guide", url: strings.guideURL)
        externalLink("GitHub", url: URL(string: "https://github.com/getcortexlabs/rove")!)
      }.frame(maxWidth: .infinity)

      Link(destination: URL(string: "https://getcortexlabs.com/")!) {
        HStack(spacing: 6) {
          Text(strings("A project by"))
          Text(verbatim: "Cortex Labs").fontWeight(.semibold).foregroundStyle(Lab.secondary)
          Image(systemName: "arrow.up.right").font(.system(size: 8, weight: .medium))
        }.font(.system(size: 11)).foregroundStyle(Lab.muted)
          .padding(.vertical, 4).frame(maxWidth: .infinity)
          .contentShape(Rectangle())
      }.buttonStyle(.plain)
    }
    .padding(30).frame(width: 500)
    .background { WorkspaceBackdrop() }.foregroundStyle(Lab.ink)
    .preferredColorScheme(.dark).tint(Lab.accent)
    .environment(\.locale, strings.locale)
    .environment(
      \.visualComfort,
      VisualComfort(
        reduceMotion: preferences.reduceMotion, reduceTransparency: preferences.reduceTransparency)
    )
    .onExitCommand { dismissWindow(id: "about") }
  }

  private var identity: some View {
    HStack(spacing: 24) {
      ZStack {
        RoundedRectangle(cornerRadius: 25)
          .fill(
            LinearGradient(
              colors: [Lab.accent.opacity(0.13), Lab.ice.opacity(0.04)],
              startPoint: .topLeading, endPoint: .bottomTrailing)
          )
          .overlay(RoundedRectangle(cornerRadius: 25).strokeBorder(.white.opacity(0.09)))
        if let url = Lab.resource("RoveLogo", extension: "png"),
          let image = NSImage(contentsOf: url)
        {
          Image(nsImage: image).resizable().interpolation(.high).scaledToFit().padding(15)
            .shadow(color: Lab.accent.opacity(0.16), radius: 13, y: 3)
        }
      }.frame(width: 104, height: 112).accessibilityHidden(true)

      VStack(alignment: .leading, spacing: 7) {
        Text(verbatim: "Rove").font(Lab.display(44)).tracking(-1.5)
        Text(strings("Explore evolutionary paths."))
          .font(.system(size: 12)).foregroundStyle(Lab.secondary)
        Text(version.map { strings("Preview · %@", $0) } ?? strings("Development build"))
          .font(.system(size: 10, weight: .medium, design: .monospaced))
          .foregroundStyle(Lab.accent).padding(.horizontal, 10).padding(.vertical, 5)
          .background(Lab.accent.opacity(0.08), in: Capsule())
          .overlay(Capsule().strokeBorder(Lab.accent.opacity(0.15), lineWidth: 0.75))
          .padding(.top, 3).textSelection(.enabled)
      }
    }
  }

  private func credit(_ name: String, initials: String, role: String, color: Color) -> some View {
    HStack(spacing: 14) {
      Text(verbatim: initials).font(Lab.display(15, weight: .semibold)).foregroundStyle(color)
        .frame(width: 40, height: 40)
        .background(color.opacity(0.08), in: RoundedRectangle(cornerRadius: 13))
        .overlay(
          RoundedRectangle(cornerRadius: 13).strokeBorder(color.opacity(0.13), lineWidth: 0.75)
        ).accessibilityHidden(true)
      VStack(alignment: .leading, spacing: 5) {
        Text(verbatim: name).font(.system(size: 14, weight: .semibold))
        Text(strings(role)).font(.system(size: 11)).foregroundStyle(Lab.secondary)
      }
      Spacer(minLength: 0)
    }.accessibilityElement(children: .combine)
  }

  private func externalLink(_ title: String, url: URL) -> some View {
    Link(destination: url) {
      HStack(spacing: 9) {
        Text(strings(title))
        Image(systemName: "arrow.up.right").font(.system(size: 9, weight: .semibold))
      }
    }.buttonStyle(LabButtonStyle(compact: true))
      .help(url.absoluteString)
  }
}
