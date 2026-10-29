import SwiftUI

struct GroupSection: View {
  @Bindable var workspace: Workspace
  @State private var editing = false
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 14) {
        HStack {
          Text(strings("Groups that must stay together")).font(.system(size: 13, weight: .semibold))
          Spacer()
          Button {
            editing = true
          } label: {
            Label(strings("Add group"), systemImage: "plus")
          }.buttonStyle(LabButtonStyle(compact: true))
        }
        Text(
          strings(
            "A constrained group must form a clade. Its support is imposed by your choice, not independently recovered from the data."
          )
        )
        .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
        if workspace.monophyly.isEmpty {
          Text(strings("No groups constrained. Rove does not select an ingroup automatically."))
            .font(.system(size: 11)).foregroundStyle(Lab.muted)
        }
        ForEach(Array(workspace.monophyly.enumerated()), id: \.offset) { index, group in
          HStack(alignment: .top) {
            Image(systemName: "point.3.connected.trianglepath.dotted").foregroundStyle(Lab.lilac)
            Text(group.joined(separator: ", ")).font(.system(size: 12)).textSelection(.enabled)
            Spacer()
            Button {
              workspace.monophyly.remove(at: index)
            } label: {
              Image(systemName: "minus.circle")
            }.buttonStyle(.plain).accessibilityLabel(strings("Remove group %@", String(index + 1)))
          }
        }
      }
    }
    .sheet(isPresented: $editing) { GroupEditor(workspace: workspace) }
  }
}

private struct GroupEditor: View {
  @Bindable var workspace: Workspace
  @Environment(\.dismiss) private var dismiss
  @State private var selected: Set<String> = []
  @State private var search = ""
  private var strings: AppStrings { workspace.strings }
  private var taxa: [String] { workspace.report?.allTaxa ?? [] }
  private var group: [String] { taxa.filter { selected.contains($0) } }
  private var issue: String? {
    GroupConstraints.validate(workspace.monophyly + [group], taxa: taxa)
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 18) {
      Text(strings("Keep a group together")).font(Lab.display(26))
      Text(strings("Select the terminals that must share an exclusive ancestor."))
        .font(.system(size: 12)).foregroundStyle(Lab.secondary)
      TextField(strings("Find a terminal"), text: $search).textFieldStyle(.roundedBorder)
      HStack {
        Button(strings("Select all")) { selected = Set(taxa) }
        Button(strings("Clear")) { selected = [] }
        Spacer()
        Text(strings("%@ selected", strings.number(selected.count))).foregroundStyle(Lab.secondary)
      }.font(.system(size: 12))
      ScrollView {
        LazyVStack(alignment: .leading, spacing: 10) {
          ForEach(
            taxa.filter { search.isEmpty || $0.localizedCaseInsensitiveContains(search) },
            id: \.self
          ) { taxon in
            Toggle(
              taxon,
              isOn: Binding(
                get: { selected.contains(taxon) },
                set: { if $0 { selected.insert(taxon) } else { selected.remove(taxon) } }
              )
            ).toggleStyle(.checkbox)
          }
        }.frame(maxWidth: .infinity, alignment: .leading).padding(12)
      }.frame(height: 260).background(Lab.surface, in: RoundedRectangle(cornerRadius: 12))
      if let issue { Text(strings(issue)).font(.system(size: 11)).foregroundStyle(Lab.secondary) }
      HStack {
        Button(strings("Cancel")) { dismiss() }.keyboardShortcut(.cancelAction)
        Spacer()
        Button(strings("Add group")) {
          workspace.monophyly.append(group)
          dismiss()
        }.keyboardShortcut(.defaultAction).disabled(issue != nil)
      }
    }.padding(28).frame(width: 470).foregroundStyle(Lab.ink).background(Lab.background)
      .preferredColorScheme(.dark).tint(Lab.accent)
  }
}
