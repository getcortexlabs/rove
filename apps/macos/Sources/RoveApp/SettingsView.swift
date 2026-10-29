import SwiftUI

struct SettingsView: View {
  @Bindable var preferences: AppPreferences
  private var strings: AppStrings { preferences.strings }

  var body: some View {
    VStack(alignment: .leading, spacing: 25) {
      HStack(spacing: 13) {
        Image(systemName: "slider.horizontal.3")
          .font(.system(size: 24, weight: .light)).foregroundStyle(Lab.accent)
          .frame(width: 51, height: 51)
          .background(Lab.accent.opacity(0.08), in: RoundedRectangle(cornerRadius: 15))
        VStack(alignment: .leading, spacing: 4) {
          Text(strings("Make yourself at home.")).font(Lab.display(26, weight: .semibold))
          Text(strings("A few things, just the way you like them."))
            .font(.system(size: 12)).foregroundStyle(Lab.secondary)
        }
      }

      LabPanel {
        VStack(alignment: .leading, spacing: 15) {
          Label(strings("Language"), systemImage: "globe")
            .font(.system(size: 13, weight: .semibold))
          Picker(strings("App language"), selection: $preferences.language) {
            Text(strings("Follow your Mac")).tag(AppLanguage.system)
            Text(verbatim: "English").tag(AppLanguage.english)
            Text(verbatim: "Português (Brasil)").tag(AppLanguage.portuguese)
          }.pickerStyle(.menu)
          Text(
            strings(
              "Changes apply immediately. Your open matrix and analysis settings stay in place.")
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
        }
      }

      LabPanel {
        VStack(alignment: .leading, spacing: 18) {
          Label(strings("Visual comfort"), systemImage: "eye")
            .font(.system(size: 13, weight: .semibold))
          Toggle(isOn: $preferences.reduceMotion) {
            option("Reduce animations", detail: "Keep the explorer and transitions still.")
          }.toggleStyle(.switch)
          Rectangle().fill(Lab.border.opacity(0.7)).frame(height: 1)
          Toggle(isOn: $preferences.reduceTransparency) {
            option("Reduce transparency", detail: "Give the sidebar a solid background.")
          }.toggleStyle(.switch)
          Text(strings("Your Mac's accessibility preferences always take priority."))
            .font(.system(size: 11)).foregroundStyle(Lab.muted).lineSpacing(3)
        }
      }

      HStack(spacing: 7) {
        Image(systemName: "internaldrive").foregroundStyle(Lab.ice)
        Text(strings("Saved automatically, only on this Mac."))
        Spacer()
        Text(verbatim: "rove").font(Lab.display(17, weight: .semibold)).foregroundStyle(Lab.accent)
      }.font(.system(size: 10)).foregroundStyle(Lab.muted)
    }
    .padding(30).frame(width: 560)
    .background { WorkspaceBackdrop() }.foregroundStyle(Lab.ink)
    .preferredColorScheme(.dark).tint(Lab.accent)
    .environment(\.locale, strings.locale)
    .environment(
      \.visualComfort,
      VisualComfort(
        reduceMotion: preferences.reduceMotion, reduceTransparency: preferences.reduceTransparency)
    )
    .navigationTitle(strings("Settings"))
  }

  private func option(_ title: String, detail: String) -> some View {
    VStack(alignment: .leading, spacing: 5) {
      Text(strings(title)).font(.system(size: 12, weight: .medium))
      Text(strings(detail)).font(.system(size: 11)).foregroundStyle(Lab.secondary)
    }.frame(maxWidth: .infinity, alignment: .leading)
  }
}
