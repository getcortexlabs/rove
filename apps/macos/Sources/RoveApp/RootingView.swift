import SwiftUI

struct RootingSection: View {
  @Bindable var workspace: Workspace
  private var strings: AppStrings { workspace.strings }
  private var taxa: [String] { workspace.report?.allTaxa ?? [] }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 16) {
        HStack {
          Label(strings("Outgroup"), systemImage: "arrow.triangle.branch")
            .font(.system(size: 13, weight: .semibold))
          Spacer()
          Picker(strings("Outgroup"), selection: $workspace.outgroup) {
            Text(strings("Not selected")).tag(String?.none)
            ForEach(taxa, id: \.self) { taxon in
              Text(taxon).tag(Optional(taxon))
            }
          }.labelsHidden().frame(maxWidth: 220)
        }
        Text(
          strings(
            "Choose one terminal outside the group you want to study. Every terminal stays in the analysis."
          )
        )
        .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
        if let outgroup = workspace.outgroup {
          HStack(alignment: .top, spacing: 12) {
            Image(systemName: "point.3.connected.trianglepath.dotted")
              .font(.system(size: 22)).foregroundStyle(Lab.ice).accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 8) {
              Text(
                strings(
                  "%@ on one side of the root; the other %@ terminals together on the other.",
                  outgroup, strings.number(taxa.count - 1))
              )
              .font(.system(size: 13, weight: .medium))
              Text(
                strings(
                  "The outgroup is a terminal, not the ancestor at the root. This choice constrains the ingroup; its support is imposed by the setup."
                )
              )
              .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
            }
          }.padding(16).background(Lab.ice.opacity(0.045), in: RoundedRectangle(cornerRadius: 12))
        } else {
          Text(strings("No outgroup selected. Any applied groups below still constrain the tree."))
            .font(.system(size: 11)).foregroundStyle(Lab.secondary)
        }
        if let issue = GroupConstraints.validate(
          workspace.monophyly, taxa: taxa, outgroup: workspace.outgroup)
        {
          InlineMessage(text: strings(issue), isError: true)
        }
      }
    }
  }
}

struct ConstraintSummary: View {
  let outgroup: String?
  let taxa: [String]?
  let groups: [[String]]
  let strings: AppStrings

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack(alignment: .top) {
        Text(strings("Outgroup")).foregroundStyle(Lab.secondary)
        Spacer()
        Text(outgroup ?? strings("Not selected")).textSelection(.enabled)
      }.font(.system(size: 12))
      if let outgroup, let taxa {
        Text(
          strings(
            "%@ on one side of the root; the other %@ terminals together on the other.",
            outgroup, strings.number(taxa.count - 1))
        )
        .font(.system(size: 12)).foregroundStyle(Lab.ice).lineSpacing(3)
      }
      if groups.isEmpty {
        Text(strings("No groups applied. Root placement is unconstrained by groups."))
          .font(.system(size: 12)).foregroundStyle(Lab.secondary)
      } else {
        Text(strings("Applied groups")).font(.system(size: 12, weight: .medium))
        ForEach(Array(groups.enumerated()), id: \.offset) { _, group in
          Text(group.joined(separator: ", ")).font(.system(size: 11)).textSelection(.enabled)
        }
        Text(
          strings("Support for an imposed group is not independent evidence from the characters.")
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
      }
      if let taxa {
        DisclosureGroup(strings("All %@ terminals retained", strings.number(taxa.count))) {
          Text(taxa.joined(separator: ", ")).font(.system(size: 11)).textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading).padding(.top, 8)
        }.font(.system(size: 12)).foregroundStyle(Lab.secondary)
      } else {
        Text(
          strings(
            "This older setup did not record its full terminal list. Reopen it to check the original matrix."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary)
      }
    }
  }
}
