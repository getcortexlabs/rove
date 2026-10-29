import SwiftUI

struct GroupSection: View {
  @Bindable var workspace: Workspace
  @State private var editing: GroupSelection?
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 14) {
        HStack {
          Text(strings("Additional groups")).font(.system(size: 13, weight: .semibold))
          Spacer()
          Button {
            editing = GroupSelection()
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
          Text(strings("No additional groups applied."))
            .font(.system(size: 11)).foregroundStyle(Lab.muted)
        }
        ForEach(Array(workspace.monophyly.enumerated()), id: \.offset) { index, group in
          HStack(alignment: .top) {
            Image(systemName: "point.3.connected.trianglepath.dotted").foregroundStyle(Lab.lilac)
            Text(group.joined(separator: ", ")).font(.system(size: 12)).textSelection(.enabled)
            Spacer()
            Button {
              editing = GroupSelection(index: index, group: group)
            } label: {
              Image(systemName: "pencil")
            }.buttonStyle(.plain).accessibilityLabel(strings("Edit group %@", String(index + 1)))
            Button {
              workspace.monophyly.remove(at: index)
            } label: {
              Image(systemName: "minus.circle")
            }.buttonStyle(.plain).accessibilityLabel(strings("Remove group %@", String(index + 1)))
          }
        }
      }
    }
    .sheet(item: $editing) { selection in
      GroupEditor(workspace: workspace, selection: selection)
    }
  }
}

struct GroupEditor: View {
  @Bindable var workspace: Workspace
  @Environment(\.dismiss) private var dismiss
  @State private var selection: GroupSelection
  @State private var search = ""
  init(workspace: Workspace, selection: GroupSelection) {
    self.workspace = workspace
    _selection = State(initialValue: selection)
  }

  private var strings: AppStrings { workspace.strings }
  private var taxa: [String] { workspace.report?.allTaxa ?? [] }
  private var issue: String? {
    GroupConstraints.validate(
      selection.groups(replacing: workspace.monophyly, taxa: taxa), taxa: taxa,
      outgroup: workspace.outgroup)
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 18) {
      Text(strings("Keep a group together")).font(Lab.display(26))
      Text(
        strings(
          "Select the terminals that must share an exclusive ancestor. Unchecked terminals stay in the analysis."
        )
      )
      .font(.system(size: 12)).foregroundStyle(Lab.secondary)
      TextField(strings("Find a terminal"), text: $search).textFieldStyle(.roundedBorder)
      HStack {
        Button(strings("Select all")) { selection.selected = Set(taxa) }
        Button(strings("Clear")) { selection.selected = [] }
        Spacer()
        Text(strings("%@ selected", strings.number(selection.selected.count))).foregroundStyle(
          Lab.secondary)
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
                get: { selection.selected.contains(taxon) },
                set: {
                  if $0 {
                    selection.selected.insert(taxon)
                  } else {
                    selection.selected.remove(taxon)
                  }
                }
              )
            ).toggleStyle(.checkbox)
          }
        }.frame(maxWidth: .infinity, alignment: .leading).padding(12)
      }.frame(height: 260).background(Lab.surface, in: RoundedRectangle(cornerRadius: 12))
      if let issue { Text(strings(issue)).font(.system(size: 11)).foregroundStyle(Lab.secondary) }
      Text(strings("Changes are applied only when you press Apply group."))
        .font(.system(size: 11)).foregroundStyle(Lab.secondary)
      HStack {
        Button(strings("Cancel")) { dismiss() }.keyboardShortcut(.cancelAction)
        Spacer()
        Button(strings("Apply group")) {
          if workspace.applyGroup(selection) { dismiss() }
        }.keyboardShortcut(.defaultAction).disabled(issue != nil)
      }
    }.padding(28).frame(width: 470).foregroundStyle(Lab.ink).background(Lab.background)
      .preferredColorScheme(.dark).tint(Lab.accent)
  }
}
