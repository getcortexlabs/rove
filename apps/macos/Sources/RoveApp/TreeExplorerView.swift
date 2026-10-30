import SwiftUI

struct TreeExplorerView: View {
  @Bindable var model: TreeExplorer
  let preferences: AppPreferences
  private var strings: AppStrings { preferences.strings }

  var body: some View {
    VStack(spacing: 0) {
      header.padding(22)
      if let tree = model.document, let layout = model.layout {
        HStack(spacing: 0) {
          VStack(spacing: 0) {
            controls.padding(12)
            GeometryReader { geometry in
              TreeCanvas(
                figure: TreeFigure(
                  tree: tree, layout: layout, viewport: geometry.size,
                  collapsed: layout.collapsed, selected: model.selected,
                  showSupport: model.showSupport, strings: strings), model: model
              )
              .onAppear { model.viewport = geometry.size }
              .onChange(of: geometry.size) { _, size in model.viewport = size }
            }
            HStack {
              Image(systemName: "cursorarrow.motionlines")
              Text(strings("Scroll or drag to pan · Pinch to zoom"))
              Spacer()
              if model.arranging { ProgressView().controlSize(.mini) }
            }.font(.system(size: 10)).foregroundStyle(Lab.secondary).padding(12)
          }
          Divider()
          inspector(tree).frame(width: 280)
        }
      } else {
        Spacer()
        if model.loading {
          ProgressView(strings("Opening annotated tree…"))
        } else if model.error == nil {
          Text(strings("No annotated tree is open."))
        }
        Spacer()
      }
      if let error = model.error {
        DiagnosticMessage(title: strings(error), details: model.details, strings: strings)
          .padding(16)
      }
    }
    .frame(minWidth: 880, minHeight: 640)
    .foregroundStyle(Lab.ink).background(WorkspaceBackdrop())
    .preferredColorScheme(.dark).tint(Lab.accent)
    .environment(
      \.visualComfort,
      VisualComfort(
        reduceMotion: preferences.reduceMotion,
        reduceTransparency: preferences.reduceTransparency)
    )
    .task { await model.load() }
    .task(id: model.query) {
      try? await Task.sleep(for: .milliseconds(150))
      guard !Task.isCancelled else { return }
      await model.search()
    }
  }

  private var header: some View {
    HStack(spacing: 16) {
      Image(systemName: "point.3.connected.trianglepath.dotted")
        .font(.system(size: 26)).foregroundStyle(Lab.lilac)
        .frame(width: 52, height: 52).background(
          Lab.lilac.opacity(0.1), in: RoundedRectangle(cornerRadius: 15))
      VStack(alignment: .leading, spacing: 5) {
        Text(strings("Follow the branches.")).font(Lab.display(26))
        Text(model.runDirectory.lastPathComponent).font(.system(size: 11, design: .monospaced))
          .foregroundStyle(Lab.secondary).lineLimit(1).truncationMode(.middle)
        if let summary = model.document?.summary {
          Text(
            "MCC · "
              + strings(
                "%@ trees retained · %@ discarded", strings.number(summary.retainedSamples),
                strings.number(summary.discardedSamples))
          )
          .font(.system(size: 11)).foregroundStyle(Lab.ice)
        }
      }
      Spacer(minLength: 8)
      Button {
        model.exportFigure(strings: strings)
      } label: {
        Label(strings("Export SVG"), systemImage: "square.and.arrow.up")
      }.buttonStyle(LabButtonStyle(compact: true))
        .disabled(model.document == nil || model.arranging || model.exporting)
    }
  }

  private var controls: some View {
    HStack(spacing: 12) {
      Toggle(strings("Support"), isOn: $model.showSupport).toggleStyle(.checkbox)
      Spacer(minLength: 4)
      Button {
        model.zoom = max(1, model.zoom / 1.25)
      } label: {
        Image(systemName: "minus.magnifyingglass")
      }
      .help(strings("Zoom out")).accessibilityLabel(strings("Zoom out"))
      .disabled(model.zoom <= 1)
      Text("\(Int(model.zoom * 100))%")
        .font(.system(size: 11, design: .monospaced)).frame(width: 42)
      Button {
        model.zoom = min(4, model.zoom * 1.25)
      } label: {
        Image(systemName: "plus.magnifyingglass")
      }
      .help(strings("Zoom in")).accessibilityLabel(strings("Zoom in"))
      .disabled(model.zoom >= 4)
      Button(strings("Fit width")) { model.zoom = 1 }
      Button {
        model.resetDisplay()
      } label: {
        Image(systemName: "arrow.counterclockwise")
      }
      .help(strings("Reset display")).accessibilityLabel(strings("Reset display"))
      .disabled(model.arranging)
    }.font(.system(size: 11)).buttonStyle(.borderless)
  }

  private func inspector(_ tree: TreeDocument) -> some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 18) {
        HStack {
          Image(systemName: "magnifyingglass").foregroundStyle(Lab.muted)
          TextField(strings("Find a taxon"), text: $model.query).textFieldStyle(.plain)
            .onSubmit { if let id = model.matches.first { model.reveal(id) } }
          if !model.query.isEmpty {
            Button {
              model.query = ""
            } label: {
              Image(systemName: "xmark.circle.fill")
            }
            .buttonStyle(.plain).accessibilityLabel(strings("Clear search"))
          }
        }.padding(10).background(Lab.raised, in: RoundedRectangle(cornerRadius: 9))
        if !model.query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
          VStack(alignment: .leading, spacing: 8) {
            Text(strings("%@ matches · showing up to 50", strings.number(model.matchCount)))
              .font(.system(size: 10)).foregroundStyle(Lab.secondary)
            ForEach(model.matches, id: \.self) { id in
              Button {
                model.reveal(id)
              } label: {
                Label(tree.nodes[id].taxon ?? "", systemImage: "leaf")
                  .frame(maxWidth: .infinity, alignment: .leading)
              }.buttonStyle(.plain).foregroundStyle(Lab.ice)
            }
          }
        }
        HStack {
          Eyebrow(text: strings("Node inspector"), color: Lab.ice)
          Spacer()
          Button {
            model.moveSelection(-1)
          } label: {
            Image(systemName: "chevron.up")
          }
          .accessibilityLabel(strings("Previous node"))
          Button {
            model.moveSelection(1)
          } label: {
            Image(systemName: "chevron.down")
          }
          .accessibilityLabel(strings("Next node"))
        }.buttonStyle(.borderless)
        if let id = model.selected {
          nodeDetails(tree.nodes[id], tree: tree)
        } else {
          Text(strings("Select a node or a tip to inspect its support, heights and descendants."))
            .foregroundStyle(Lab.secondary).lineSpacing(4)
        }
        Divider()
        provenance(tree)
        Text(strings("Support: retained clade frequency. Diamonds: constrained groups."))
          .foregroundStyle(Lab.secondary).lineSpacing(3)
        Text(strings("A summary does not establish convergence."))
          .foregroundStyle(Lab.secondary).lineSpacing(3)
        if let exported = model.exportedFile {
          Button {
            NSWorkspace.shared.activateFileViewerSelecting([exported])
          } label: {
            Label(strings("Show exported figure"), systemImage: "folder")
          }.buttonStyle(.borderless)
        }
      }.padding(18)
    }.font(.system(size: 11)).background(Lab.surface.opacity(0.7))
  }

  private func nodeDetails(_ node: DisplayTreeNode, tree: TreeDocument) -> some View {
    VStack(alignment: .leading, spacing: 12) {
      Text(node.taxon ?? strings(node.id == tree.root ? "Root" : "Internal node"))
        .font(Lab.display(22)).textSelection(.enabled)
      if node.taxon == model.outgroup, node.taxon != nil {
        Text(strings("Outgroup · a tip, not the root node.")).foregroundStyle(Lab.ice)
      }
      if node.constrained {
        Label(strings("Imposed monophyly"), systemImage: "diamond.fill").foregroundStyle(Lab.coral)
        Text(
          strings(
            "This group was constrained during inference. Its support is not independent evidence.")
        )
        .foregroundStyle(Lab.secondary).fixedSize(horizontal: false, vertical: true)
      }
      metric("Posterior support", node.posterior)
      metric("Displayed height", node.height)
      metric("Incoming branch", node.parent == nil ? nil : node.length)
      Text(
        model.document?.summary.timeUnit.map {
          strings("Heights and branches are measured in %@.", $0)
        }
          ?? strings("Heights and branches use source time units, not substitutions.")
      )
      .foregroundStyle(Lab.secondary).fixedSize(horizontal: false, vertical: true)
      if node.children != nil {
        HStack(spacing: 12) {
          Button(strings(model.collapsed.contains(node.id) ? "Expand clade" : "Collapse clade")) {
            model.toggleCollapse()
          }
          Button(strings("Rotate")) { model.rotate() }
        }.buttonStyle(.borderless).disabled(model.arranging)
      }
      DisclosureGroup(strings("Height and rate ranges")) {
        VStack(alignment: .leading, spacing: 10) {
          metric("Conditional mean height", node.heightMean)
          interval("Conditional height range", node.heightRange)
          metric("Common ancestor mean", node.ancestorMean)
          interval("Common ancestor range", node.ancestorRange)
          metric("Mean branch rate", node.rate)
          interval("Branch rate range", node.rateRange)
          Text(
            strings(
              "Ranges are minima and maxima, not credible intervals. Conditional heights and rates use trees containing this clade; common ancestor heights use all retained trees. Rates are separate multipliers."
            )
          )
          .foregroundStyle(Lab.secondary).fixedSize(horizontal: false, vertical: true)
        }.padding(.top, 10)
      }
      if let parent = node.parent {
        Button(strings("Select parent")) { model.reveal(parent) }.buttonStyle(.borderless)
      }
      if let children = node.children {
        DisclosureGroup(strings("Descendant taxa")) {
          VStack(alignment: .leading, spacing: 7) {
            ForEach(children, id: \.self) { child in
              Button {
                model.reveal(child)
              } label: {
                Label(
                  tree.nodes[child].taxon
                    ?? strings("%@ taxa", strings.number(model.layout?.tipCounts[child] ?? 0)),
                  systemImage: "arrow.turn.down.right")
              }.buttonStyle(.borderless)
            }
            Text(strings("Select a child branch to explore its descendants."))
              .foregroundStyle(Lab.secondary).fixedSize(horizontal: false, vertical: true)
            LazyVStack(alignment: .leading, spacing: 6) {
              ForEach(
                tree.descendants(of: node.id).filter { tree.nodes[$0].taxon != nil }, id: \.self
              ) { tip in
                Button(tree.nodes[tip].taxon ?? "") { model.reveal(tip) }.buttonStyle(.borderless)
              }
            }
          }.padding(.top, 8)
        }
      }
    }
  }

  private func metric(_ title: String, _ value: Double?) -> some View {
    HStack(alignment: .firstTextBaseline) {
      Text(strings(title)).foregroundStyle(Lab.secondary)
      Spacer(minLength: 6)
      Text(value.map(TreeFigure.number) ?? strings("Unavailable"))
        .font(.system(size: 11, weight: .medium, design: .monospaced)).textSelection(.enabled)
    }
  }

  private func interval(_ title: String, _ range: [Double]?) -> some View {
    VStack(alignment: .leading, spacing: 3) {
      Text(strings(title)).foregroundStyle(Lab.secondary)
      Text(
        range.map { $0.map(TreeFigure.number).joined(separator: " – ") } ?? strings("Unavailable")
      )
      .font(.system(size: 11, design: .monospaced)).textSelection(.enabled)
    }
  }

  private func provenance(_ tree: TreeDocument) -> some View {
    DisclosureGroup(strings("About this summary")) {
      VStack(alignment: .leading, spacing: 10) {
        Text(
          "MCC · "
            + strings(
              tree.summary.heightPolicy == "ca" ? "Common ancestor mean" : "Keep selected tree"))
        let burn = tree.summary.burnin
        Text(
          strings(
            "Burn-in: %@",
            burn.unit == "percent"
              ? "\(burn.value)%" : strings("%@ samples", strings.number(burn.value))))
        Text(strings("Source sample: %@", "STATE_\(tree.summary.selectedState)"))
        Text(tree.summary.sourcePath).textSelection(.enabled).foregroundStyle(Lab.secondary)
        ForEach(tree.constraints, id: \.self) { group in
          Label(group.joined(separator: ", "), systemImage: "diamond.fill")
            .foregroundStyle(Lab.coral).textSelection(.enabled)
        }
        Text(
          strings(
            "Rotating or collapsing branches changes only the display. Vertical tip order does not define the root."
          )
        )
        .foregroundStyle(Lab.secondary)
      }.fixedSize(horizontal: false, vertical: true).padding(.top, 10)
    }
  }
}
