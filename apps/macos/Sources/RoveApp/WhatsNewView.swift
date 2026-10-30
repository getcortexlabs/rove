import SwiftUI

struct WhatsNewView: View {
  @Bindable var preferences: AppPreferences
  let releases: [RoveRelease]
  let initial: RoveRelease
  var canOpenExample = true
  let onDone: () -> Void
  let onAction: (ReleaseHighlight.Destination) -> Void
  @State private var selection: String?
  @State private var expanded: Set<String> = []
  @State private var greeted = false
  @Environment(\.visualComfort) private var comfort
  private var strings: AppStrings { preferences.strings }
  private var release: RoveRelease { releases.first { $0.version == selection } ?? initial }

  var body: some View {
    VStack(spacing: 0) {
      HStack {
        Label(strings("FIELD NOTES"), systemImage: "sparkles")
          .font(.system(size: 10, weight: .medium, design: .monospaced)).tracking(1.4)
          .foregroundStyle(Lab.ice)
        Spacer()
        Picker(
          strings("Version"),
          selection: Binding(
            get: { release.version },
            set: {
              selection = $0
              expanded = []
            })
        ) {
          ForEach(releases) { Text(verbatim: "\($0.version) · Beta").tag($0.version) }
        }.labelsHidden().pickerStyle(.menu).frame(width: 130).controlSize(.small)
        Button(action: onDone) { Image(systemName: "xmark").font(.system(size: 12)) }
          .buttonStyle(.plain).foregroundStyle(Lab.muted).padding(.leading, 12)
          .accessibilityLabel(strings("Close"))
      }.padding(.horizontal, 30).padding(.top, 25).padding(.bottom, 10)

      ScrollView {
        VStack(alignment: .leading, spacing: 24) {
          HStack(alignment: .center, spacing: 14) {
            VStack(alignment: .leading, spacing: 13) {
              Text(strings("What's new in Rove")).font(.system(size: 12)).foregroundStyle(Lab.muted)
              Text(strings(release.headline)).font(Lab.display(37)).tracking(-1.2)
                .fixedSize(horizontal: false, vertical: true)
              Text(strings(release.introduction)).font(.system(size: 13)).foregroundStyle(
                Lab.secondary
              )
              .lineSpacing(5).fixedSize(horizontal: false, vertical: true)
            }.frame(maxWidth: .infinity, alignment: .leading)
            Button {
              withAnimation(
                comfort.reduceMotion ? nil : .spring(response: 0.4, dampingFraction: 0.5)
              ) { greeted.toggle() }
            } label: {
              if let image = Lab.explorerImage {
                Image(nsImage: image).resizable().scaledToFit().frame(width: 145, height: 145)
                  .rotationEffect(.degrees(comfort.reduceMotion ? 0 : (greeted ? -8 : 3)))
                  .scaleEffect(greeted && !comfort.reduceMotion ? 1.06 : 1)
              }
            }.buttonStyle(.plain).accessibilityLabel(strings("Say hello to Rove's fossil explorer"))
          }.padding(.bottom, 4)

          ForEach(release.highlights) { highlight in
            VStack(alignment: .leading, spacing: 14) {
              Button {
                withAnimation(comfort.reduceMotion ? nil : .easeInOut(duration: 0.2)) {
                  if !expanded.insert(highlight.id).inserted { expanded.remove(highlight.id) }
                }
              } label: {
                HStack(alignment: .top, spacing: 16) {
                  Image(systemName: highlight.symbol).font(.system(size: 20, weight: .light))
                    .foregroundStyle(Lab.accent)
                    .frame(width: 43, height: 43).background(
                      Lab.accent.opacity(0.07), in: RoundedRectangle(cornerRadius: 13))
                  VStack(alignment: .leading, spacing: 7) {
                    Text(strings(highlight.title)).font(Lab.display(20, weight: .semibold))
                      .tracking(-0.3)
                    Text(strings(highlight.summary)).font(.system(size: 12)).foregroundStyle(
                      Lab.secondary
                    ).lineSpacing(3)
                  }.frame(maxWidth: .infinity, alignment: .leading)
                  Image(systemName: expanded.contains(highlight.id) ? "minus" : "plus")
                    .font(.system(size: 11)).foregroundStyle(Lab.muted).padding(.top, 5)
                }.contentShape(Rectangle())
              }.buttonStyle(.plain)
                .accessibilityValue(
                  strings(expanded.contains(highlight.id) ? "Expanded" : "Collapsed"))
              if expanded.contains(highlight.id) {
                Text(strings(highlight.detail)).font(.system(size: 12)).foregroundStyle(
                  Lab.secondary
                )
                .lineSpacing(5).fixedSize(horizontal: false, vertical: true)
                Button {
                  onAction(highlight.destination)
                } label: {
                  Label(strings(highlight.action), systemImage: "arrow.up.right")
                }.buttonStyle(.plain).font(.system(size: 12, weight: .medium)).foregroundStyle(
                  Lab.ice
                )
                .disabled(highlight.destination == .example && !canOpenExample)
                if highlight.destination == .example && !canOpenExample {
                  Text(strings("Start a new workspace to open the example."))
                    .font(.system(size: 11)).foregroundStyle(Lab.muted)
                }
              }
            }.padding(21).frame(maxWidth: .infinity, alignment: .leading)
              .background { ArrivalCard() }
          }
          Text(strings("New paths. Same curious spirit."))
            .font(.system(size: 11)).foregroundStyle(Lab.muted).frame(maxWidth: .infinity)
        }.padding(.horizontal, 30).padding(.vertical, 16)
      }
      HStack {
        Text(verbatim: "Cortex Labs · Rove").font(.system(size: 10)).foregroundStyle(Lab.muted)
        Spacer()
        Button(strings("Back to exploring"), action: onDone).buttonStyle(
          LabButtonStyle(prominent: true, compact: true)
        )
        .keyboardShortcut(.defaultAction)
      }.padding(24)
    }
    .frame(
      minWidth: 620, idealWidth: 700, maxWidth: 820, minHeight: 550, idealHeight: 740,
      maxHeight: 850
    )
    .background { WorkspaceBackdrop() }.foregroundStyle(Lab.ink)
    .preferredColorScheme(.dark).tint(Lab.accent)
    .onExitCommand(perform: onDone)
  }
}
