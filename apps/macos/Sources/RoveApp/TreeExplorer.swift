import AppKit
import Observation
import UniformTypeIdentifiers

@MainActor @Observable
final class TreeExplorer: Identifiable {
  let id = UUID()
  let directory: URL
  let runDirectory: URL
  let executable: URL
  private(set) var document: TreeDocument?
  private(set) var layout: TreeLayout?
  private(set) var loading = false
  private(set) var arranging = false
  private(set) var error: String?
  private(set) var details: String?
  private(set) var outgroup: String?
  private(set) var collapsed: Set<Int> = []
  private(set) var rotated: Set<Int> = []
  var selected: Int?
  var query = ""
  private(set) var matches: [Int] = []
  private(set) var matchCount = 0
  var showSupport = true
  var zoom = 1.0
  var viewport = CGSize(width: 640, height: 540)
  private(set) var focusRevision = 0
  private(set) var exporting = false
  private(set) var exportedFile: URL?
  /// The run folder's name, or the tree file's name for an external sample.
  private(set) var runName: String
  private var revision = 0
  private var arrangement: Task<Void, Never>?

  init(directory: URL, runDirectory: URL, executable: URL) {
    self.directory = directory
    self.runDirectory = runDirectory
    self.executable = executable
    runName = runDirectory.lastPathComponent
  }

  func load() async {
    guard document == nil, !loading else { return }
    loading = true
    defer { loading = false }
    let directory = directory
    let executable = executable
    let run = runDirectory
    do {
      let (tree, layout, group, name) = try await Task.detached {
        let tree = try TreeInspector.read(directory: directory, executable: executable)
        let layout = TreeLayout.build(tree, collapsed: [], rotated: [])
        let entry = HistoryEntry.read(run)
        let draft = entry.draft
        let groups = Set(tree.constraints.map { $0.sorted().joined(separator: "\u{0}") })
        let applied = Set((draft?.constraints ?? []).map { $0.sorted().joined(separator: "\u{0}") })
        let outgroup =
          draft?.matrixSHA256 == tree.summary.matrixSha256 && groups == applied
          ? draft?.outgroup : nil
        return (tree, layout, outgroup, entry.source?.name ?? run.lastPathComponent)
      }.value
      try Task.checkCancellation()
      document = tree
      self.layout = layout
      outgroup = group
      runName = name
    } catch is CancellationError {
    } catch {
      self.error = "The annotated tree could not be opened."
      details = error.localizedDescription
    }
  }

  func toggleCollapse() {
    guard let id = selected, document?.nodes[id].children != nil else { return }
    if !collapsed.insert(id).inserted { collapsed.remove(id) }
    rebuild()
  }

  func rotate() {
    guard let id = selected, document?.nodes[id].children != nil else { return }
    if !rotated.insert(id).inserted { rotated.remove(id) }
    rebuild()
  }

  func resetDisplay() {
    collapsed.removeAll()
    rotated.removeAll()
    zoom = 1
    rebuild()
  }

  private func rebuild() {
    guard let document else { return }
    revision += 1
    let current = revision
    let collapsed = collapsed
    let rotated = rotated
    arrangement?.cancel()
    arranging = true
    arrangement = Task {
      let value = await Task.detached {
        TreeLayout.build(document, collapsed: collapsed, rotated: rotated)
      }.value
      guard !Task.isCancelled, current == revision else { return }
      layout = value
      arranging = false
      focusRevision += 1
    }
  }

  func waitForLayout() async { await arrangement?.value }

  func search() async {
    guard let document else { return }
    let text = query.trimmingCharacters(in: .whitespacesAndNewlines)
    let result = await Task.detached {
      text.isEmpty
        ? []
        : document.nodes.compactMap { node in
          node.taxon?.localizedStandardContains(text) == true ? node.id : nil
        }
    }.value
    guard !Task.isCancelled, query.trimmingCharacters(in: .whitespacesAndNewlines) == text else {
      return
    }
    matches = Array(result.prefix(50))
    matchCount = result.count
  }

  func reveal(_ id: Int) {
    guard let document, document.nodes.indices.contains(id) else { return }
    selected = id
    var parent = document.nodes[id].parent
    var changed = false
    while let p = parent {
      if collapsed.remove(p) != nil { changed = true }
      parent = document.nodes[p].parent
    }
    if changed { rebuild() } else { focusRevision += 1 }
  }

  func moveSelection(_ offset: Int) {
    guard let order = layout?.order, !order.isEmpty else { return }
    let index = selected.flatMap { order.firstIndex(of: $0) } ?? (offset > 0 ? -1 : order.count)
    reveal(order[min(max(index + offset, 0), order.count - 1)])
  }

  func clearError() {
    error = nil
    details = nil
  }

  func exportFigure(strings: AppStrings) {
    guard let document, let layout, !arranging, !exporting else { return }
    let panel = NSSavePanel()
    panel.allowedContentTypes = [.svg]
    panel.canCreateDirectories = true
    panel.nameFieldStringValue = "rove-mcc.svg"
    panel.title = strings("Export tree figure")
    guard panel.runModal() == .OK, let url = panel.url else { return }
    let figure = TreeFigure(
      tree: document, layout: layout, viewport: viewport, collapsed: layout.collapsed,
      selected: selected, showSupport: showSupport, strings: strings)
    let runName = runName
    let protected =
      ["mcc.tree", "samples.trees", "summary.json"].map { directory.appendingPathComponent($0) }
      + [URL(fileURLWithPath: document.summary.sourcePath)]
    exporting = true
    error = nil
    details = nil
    Task {
      do {
        try await Task.detached {
          try figure.write(to: url, runName: runName, protectedFiles: protected)
        }.value
        exportedFile = url
      } catch {
        self.error = "The figure could not be exported."
        details = error.localizedDescription
      }
      exporting = false
    }
  }
}
