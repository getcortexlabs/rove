import SwiftUI

struct RootingSection: View {
  @Bindable var workspace: Workspace
  private var strings: AppStrings { workspace.strings }
  private var taxa: [String] { workspace.report?.allTaxa ?? [] }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 16) {
        HStack {
          Label(strings("Rooting"), systemImage: "arrow.triangle.branch")
            .font(.system(size: 13, weight: .semibold))
          Spacer()
          VStack(alignment: .leading, spacing: 5) {
            Text(strings("Taxon used for rooting")).font(.system(size: 10)).foregroundStyle(
              Lab.muted)
            Picker(strings("Taxon used for rooting"), selection: $workspace.outgroup) {
              Text(strings("Not selected")).tag(String?.none)
              ForEach(taxa, id: \.self) { taxon in
                Text(taxon).tag(Optional(taxon))
              }
            }.labelsHidden()
          }.frame(maxWidth: 220)
        }
        Text(
          strings(
            "Select one terminal to place on one side of the root. All remaining taxa are constrained together on the other side; every terminal stays in the analysis."
          )
        )
        .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
        if let outgroup = workspace.outgroup {
          RootingConstraintDiagram(taxon: outgroup, remaining: taxa.count - 1, strings: strings)
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
                  "The selected taxon is a tip, not the ancestral root. Its character states are not copied to the ancestor. Support for the group of remaining taxa is imposed by this constraint."
                )
              )
              .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
              .fixedSize(horizontal: false, vertical: true)
            }
          }.padding(16).background(Lab.ice.opacity(0.045), in: RoundedRectangle(cornerRadius: 12))
        } else {
          Text(
            strings(
              "No rooting taxon selected. Groups below can still constrain the tree. This choice is separate from including taxa or rotating the tree display."
            )
          )
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

struct RootingConstraintDiagram: View {
  let taxon: String
  let remaining: Int
  let strings: AppStrings

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(spacing: 14) {
        Canvas { context, size in
          var branches = Path()
          branches.move(to: CGPoint(x: 10, y: 48))
          branches.addLine(to: CGPoint(x: 40, y: 48))
          branches.move(to: CGPoint(x: 40, y: 18))
          branches.addLine(to: CGPoint(x: 40, y: 76))
          branches.move(to: CGPoint(x: 40, y: 18))
          branches.addLine(to: CGPoint(x: size.width - 6, y: 18))
          branches.move(to: CGPoint(x: 40, y: 76))
          branches.addLine(to: CGPoint(x: 70, y: 76))
          context.stroke(branches, with: .color(Lab.ice), lineWidth: 2)
          context.fill(
            Path(ellipseIn: CGRect(x: 6, y: 44, width: 8, height: 8)),
            with: .color(Lab.accent))
          var group = Path()
          group.move(to: CGPoint(x: 70, y: 76))
          group.addLine(to: CGPoint(x: size.width - 6, y: 53))
          group.addLine(to: CGPoint(x: size.width - 6, y: 96))
          group.closeSubpath()
          context.fill(group, with: .color(Lab.lilac.opacity(0.2)))
          context.stroke(group, with: .color(Lab.lilac), lineWidth: 1.5)
        }.frame(width: 140, height: 100).accessibilityHidden(true)
        VStack(alignment: .leading, spacing: 0) {
          Text(taxon).font(.system(size: 12, weight: .medium)).foregroundStyle(Lab.ice)
            .frame(height: 36, alignment: .center)
          Spacer(minLength: 0)
          Text(strings("Other %@ terminals", strings.number(remaining)))
            .font(.system(size: 12)).foregroundStyle(Lab.lilac)
            .frame(height: 46, alignment: .center)
        }.frame(height: 100)
      }
      Text(
        strings(
          "Constraint diagram. Branch lengths and relationships within the collapsed group are not estimated here."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted)
      .fixedSize(horizontal: false, vertical: true)
    }
    .accessibilityElement(children: .ignore)
    .accessibilityLabel(
      strings(
        "%@ on one side of the root; the other %@ terminals together on the other.", taxon,
        strings.number(remaining)))
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
        Text(strings("Taxon used for rooting")).foregroundStyle(Lab.secondary)
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
