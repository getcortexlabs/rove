import SwiftUI

struct WorkspaceSidebar: View {
  @Bindable var workspace: Workspace
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      identity.padding(.horizontal, 19).padding(.top, 43).padding(.bottom, 27)

      Button(action: workspace.reset) {
        HStack(spacing: 9) {
          Image(systemName: "plus").font(.system(size: 11, weight: .semibold))
          Text(strings("New analysis")).font(.system(size: 12, weight: .medium))
          Spacer()
          Text("⌘N").font(.system(size: 10)).foregroundStyle(Lab.muted)
        }.padding(.horizontal, 13).padding(.vertical, 12)
      }.buttonStyle(SidebarButtonStyle(outlined: true)).padding(.horizontal, 13).disabled(
        workspace.isRunning)

      sectionLabel("WORKSPACE").padding(.top, 29)
      VStack(spacing: 5) {
        ForEach(WorkspacePage.allCases, id: \.self) { page in
          navigationItem(page)
        }
      }.padding(.horizontal, 12)

      sectionLabel("EXPLORE").padding(.top, 28)
      Button(action: workspace.loadExample) {
        HStack(spacing: 11) {
          Image(systemName: "sparkles").font(.system(size: 15))
            .foregroundStyle(Lab.ice).frame(width: 25)
          VStack(alignment: .leading, spacing: 5) {
            Text(strings("Example matrix")).font(.system(size: 12, weight: .medium))
            Text(strings("20 taxa · 50 characters")).font(.system(size: 10)).foregroundStyle(
              Lab.muted)
          }
          Spacer(minLength: 0)
        }.padding(.horizontal, 12).padding(.vertical, 12)
      }.buttonStyle(SidebarButtonStyle()).padding(.horizontal, 12).disabled(workspace.isRunning)

      Spacer(minLength: 16)

      HStack(spacing: 7) {
        Image(systemName: "internaldrive").font(.system(size: 10))
        Text(strings("A little lab. Entirely yours.")).font(.system(size: 10))
      }.foregroundStyle(Lab.muted).padding(.horizontal, 21).padding(.bottom, 16)

      SettingsLink {
        HStack(spacing: 11) {
          Image(systemName: "gearshape").font(.system(size: 14)).frame(width: 22)
          Text(strings("Settings")).font(.system(size: 12, weight: .medium))
          Spacer()
          Text(verbatim: "⌘,").font(.system(size: 10)).foregroundStyle(Lab.muted)
        }.padding(.horizontal, 12).padding(.vertical, 11)
      }.buttonStyle(SidebarButtonStyle()).padding(.horizontal, 13).padding(.bottom, 8)

      Button {
        NSWorkspace.shared.open(strings.guideURL)
      } label: {
        HStack(spacing: 10) {
          Image(systemName: "book.closed").font(.system(size: 17, weight: .light))
            .foregroundStyle(Lab.accent)
          VStack(alignment: .leading, spacing: 4) {
            Text(strings("Field guide")).font(.system(size: 12, weight: .medium))
            Text(strings("Big ideas, small steps.")).font(.system(size: 10)).foregroundStyle(
              Lab.muted)
          }
          Spacer(minLength: 0)
          Image(systemName: "arrow.up.right").font(.system(size: 9)).foregroundStyle(Lab.muted)
        }.padding(13)
      }.buttonStyle(SidebarButtonStyle(outlined: true)).padding(.horizontal, 13)

      Button {
        NSWorkspace.shared.open(URL(string: "https://getcortexlabs.com/")!)
      } label: {
        HStack(spacing: 8) {
          Text("✳").font(.system(size: 18)).foregroundStyle(Lab.accent)
          Text("CORTEX LABS").font(.system(size: 9, weight: .medium, design: .monospaced))
            .tracking(1.3)
          Spacer()
          Image(systemName: "arrow.up.right").font(.system(size: 9))
        }.foregroundStyle(Lab.muted).padding(.horizontal, 20).padding(.vertical, 21)
      }.buttonStyle(.plain)
    }
    .background { SidebarGlass() }
    .shadow(color: .black.opacity(0.16), radius: 14, x: 3, y: 7)
  }

  private var identity: some View {
    HStack(spacing: 10) {
      if let url = Lab.resource("RoveLogo", extension: "png"), let logo = NSImage(contentsOf: url) {
        Image(nsImage: logo).resizable().scaledToFit().frame(width: 31, height: 36)
          .accessibilityHidden(true)
      }
      VStack(alignment: .leading, spacing: 1) {
        Text("rove").font(Lab.display(29, weight: .semibold)).tracking(-1)
        Text(strings("Follow your curiosity.")).font(.system(size: 9)).foregroundStyle(Lab.muted)
      }
      Spacer(minLength: 3)
      Text("LAB").font(.system(size: 8, weight: .medium, design: .monospaced)).tracking(0.8)
        .foregroundStyle(Lab.secondary).padding(.horizontal, 6).padding(.vertical, 4)
        .background(.white.opacity(0.055), in: Capsule())
        .overlay(Capsule().strokeBorder(.white.opacity(0.09), lineWidth: 0.5))
    }
  }

  private func sectionLabel(_ text: String) -> some View {
    Text(strings(text)).font(.system(size: 9, weight: .semibold)).tracking(1.4)
      .foregroundStyle(Lab.muted).padding(.horizontal, 23).padding(.bottom, 12)
  }

  private func navigationItem(_ page: WorkspacePage) -> some View {
    let selected = workspace.run == nil && workspace.page == page
    return Button {
      if workspace.canOpen(page) {
        workspace.dismissRun()
        workspace.page = page
      }
    } label: {
      HStack(spacing: 10) {
        Image(systemName: icon(page))
          .font(.system(size: 14, weight: selected ? .medium : .regular))
          .foregroundStyle(selected ? Lab.accent : Lab.secondary).frame(width: 25)
        Text(strings(page.rawValue)).font(
          .system(size: 12, weight: selected ? .semibold : .regular))
        Spacer()
        if workspace.report != nil && workspace.page.number > page.number {
          Image(systemName: "checkmark").font(.system(size: 10)).foregroundStyle(Lab.ice)
        } else {
          Text(String(format: "%02d", page.number))
            .font(.system(size: 9, weight: .medium, design: .monospaced))
            .foregroundStyle(selected ? Lab.accent : Lab.muted)
        }
      }.padding(.horizontal, 12).padding(.vertical, 13)
    }
    .buttonStyle(SidebarButtonStyle(selected: selected))
    .disabled(!workspace.canOpen(page))
    .accessibilityAddTraits(selected ? .isSelected : [])
  }

  private func icon(_ page: WorkspacePage) -> String {
    switch page {
    case .matrix: "square.grid.3x3"
    case .model: "slider.horizontal.3"
    case .review: "checkmark.seal"
    }
  }
}

private struct SidebarButtonStyle: ButtonStyle {
  var selected = false
  var outlined = false
  @State private var hovering = false
  @Environment(\.isEnabled) private var enabled
  @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
  @Environment(\.visualComfort) private var comfort
  private var reduceMotion: Bool { systemReduceMotion || comfort.reduceMotion }

  func makeBody(configuration: Configuration) -> some View {
    configuration.label
      .foregroundStyle(selected || outlined ? Lab.ink : Lab.secondary)
      .background(
        RoundedRectangle(cornerRadius: 11, style: .continuous)
          .fill(
            LinearGradient(
              colors: selected
                ? [Lab.accent.opacity(0.18), Lab.accent.opacity(0.07)]
                : [
                  .white.opacity(hovering ? 0.08 : (outlined ? 0.045 : 0)),
                  .white.opacity(hovering ? 0.035 : 0),
                ],
              startPoint: .topLeading, endPoint: .bottomTrailing))
      )
      .overlay(
        RoundedRectangle(cornerRadius: 11, style: .continuous)
          .strokeBorder(.white.opacity(selected ? 0.13 : (outlined ? 0.09 : 0)), lineWidth: 0.75)
      )
      .contentShape(RoundedRectangle(cornerRadius: 11))
      .opacity(enabled ? (configuration.isPressed ? 0.7 : 1) : 0.4)
      .onHover { hovering = $0 && enabled }
      .animation(reduceMotion ? nil : .easeOut(duration: 0.15), value: hovering)
  }
}
