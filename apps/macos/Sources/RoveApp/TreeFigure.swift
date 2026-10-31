import Foundation

struct TreeFigure: Sendable {
  let tree: TreeDocument
  let layout: TreeLayout
  let viewport: CGSize
  let collapsed: Set<Int>
  let selected: Int?
  let showSupport: Bool
  let strings: AppStrings

  let labelWidth: Double
  let branchWidth: Double
  let rowSpacing: Double
  let size: CGSize
  let top: Double
  let tipX: Double
  let supportLabels: Set<Int>

  init(
    tree: TreeDocument, layout: TreeLayout, viewport: CGSize, collapsed: Set<Int>,
    selected: Int?, showSupport: Bool, strings: AppStrings
  ) {
    self.tree = tree
    self.layout = layout
    self.viewport = viewport
    self.collapsed = collapsed
    self.selected = selected
    self.showSupport = showSupport
    self.strings = strings
    labelWidth = min(340, max(100, Double(layout.maxLabelCharacters) * 8))
    branchWidth = max(140, Double(viewport.width) - labelWidth - 64)
    rowSpacing = min(48, max(24, (Double(viewport.height) - 100) / Double(max(layout.rows - 1, 1))))
    size = CGSize(
      width: 64 + branchWidth + labelWidth,
      height: max(Double(viewport.height), Double(max(layout.rows - 1, 1)) * rowSpacing + 110))
    top = max(35, (Double(size.height) - 70 - Double(layout.rows - 1) * rowSpacing) / 2)
    tipX = 32 + branchWidth
    var labels = Set<Int>()
    var lastY = -Double.infinity
    for id in layout.order where showSupport && !collapsed.contains(id) {
      let node = tree.nodes[id]
      let p = layout.positions[id]!
      guard node.children != nil, node.posterior != nil, p.y * rowSpacing - lastY >= 15 else {
        continue
      }
      if let parent = node.parent, let q = layout.positions[parent], (p.x - q.x) * branchWidth < 42
      {
        continue
      }
      labels.insert(id)
      lastY = p.y * rowSpacing
    }
    supportLabels = labels
  }

  func point(_ id: Int) -> CGPoint? {
    layout.positions[id].map { CGPoint(x: 32 + $0.x * branchWidth, y: top + $0.y * rowSpacing) }
  }
  func label(_ id: Int) -> String {
    tree.nodes[id].taxon ?? strings("%@ taxa", strings.number(layout.tipCounts[id]))
  }
  func visibleLabel(_ id: Int) -> String {
    let text = label(id)
    return text.count > 44 ? String(text.prefix(31)) + "…" + String(text.suffix(10)) : text
  }
  func labelPoint(_ id: Int) -> CGPoint {
    let p = point(id)!
    return CGPoint(x: (collapsed.contains(id) ? tipX : p.x) + 10, y: p.y - 7)
  }
  func support(_ id: Int) -> String? {
    guard supportLabels.contains(id), let value = tree.nodes[id].posterior else { return nil }
    return Self.number(value)
  }
  func supportPoint(_ id: Int) -> CGPoint {
    let p = point(id)!
    return CGPoint(x: p.x + (tree.nodes[id].parent == nil ? 8 : -33), y: p.y - 20)
  }

  static func number(_ value: Double) -> String {
    String(format: "%.4g", locale: Locale(identifier: "en_US_POSIX"), value)
  }
  var scale: (value: Double, width: Double) {
    let height = tree.nodes[tree.root].height
    guard height > 0 else { return (0, 0) }
    let power = pow(10, floor(log10(height / 4)))
    guard power > 0, power.isFinite else { return (height, branchWidth) }
    let base = height / 4 / power
    let value = (base >= 5 ? 5.0 : base >= 2 ? 2.0 : 1.0) * power
    return (value, value / height * branchWidth)
  }
  var scaleLabel: String {
    guard scale.value > 0 else { return strings("All branch lengths are zero") }
    if let unit = tree.summary.timeUnit { return "\(Self.number(scale.value)) \(unit)" }
    return strings("%@ source time units", Self.number(scale.value))
  }

  func write(to url: URL, runName: String, protectedFiles: [URL]) throws {
    let destination = url.resolvingSymlinksInPath().standardizedFileURL
    guard url.pathExtension.lowercased() == "svg",
      !protectedFiles.contains(where: {
        $0.resolvingSymlinksInPath().standardizedFileURL == destination
      })
    else { throw ImportError.message("Choose a separate SVG file for the figure.") }
    try svg(runName: runName).write(to: url, atomically: true, encoding: .utf8)
  }

  func svg(runName: String) -> String {
    func escape(_ text: String) -> String {
      text.replacingOccurrences(of: "&", with: "&amp;").replacingOccurrences(of: "<", with: "&lt;")
        .replacingOccurrences(of: ">", with: "&gt;").replacingOccurrences(of: "\"", with: "&quot;")
        .replacingOccurrences(of: "'", with: "&apos;")
    }
    let paper = TreeStyle.paper
    func text(
      _ value: String, x: Double, y: Double, size: Int = 12, color: String = paper.ink.css,
      halo: Bool = false
    ) -> String {
      // A stroked copy underneath keeps support values legible across branches
      // in renderers without SVG 2 paint order.
      let underlay =
        halo
        ? "<text x=\"\(x)\" y=\"\(y)\" font-size=\"\(size)\" fill=\"none\" stroke=\"\(paper.canvas.css)\" stroke-width=\"3\">\(escape(value))</text>"
        : ""
      return underlay
        + "<text x=\"\(x)\" y=\"\(y)\" font-size=\"\(size)\" fill=\"\(color)\">\(escape(value))</text>"
    }
    let highlighted = Set(selected.map { tree.descendants(of: $0) } ?? [])
    let width = max(760, Double(size.width), Double(runName.count) * 7 + 64)
    let height = Double(size.height) + 160
    var svg = """
      <?xml version="1.0" encoding="UTF-8"?>
      <svg xmlns="http://www.w3.org/2000/svg" width="\(width)" height="\(height)" viewBox="0 0 \(width) \(height)">
      <title>\(escape(strings("Posterior tree summary")))</title>
      <metadata>Rove MCC; tree SHA-256: \(escape(tree.treeSha256)); selected sample: STATE_\(tree.summary.selectedState); source SHA-256: \(escape(tree.summary.sourceSha256)); constraints: \(escape(tree.constraints.map { $0.joined(separator: ", ") }.joined(separator: "; "))); display rotation and collapse only.</metadata>
      <rect width="100%" height="100%" fill="\(paper.canvas.css)"/>
      <g font-family="Helvetica,Arial,sans-serif" stroke-linecap="round" stroke-linejoin="round">
      """
    svg += "<text x=\"32\" y=\"30\" font-size=\"18\" font-weight=\"bold\" fill=\"\(paper.ink.css)\">Rove · MCC</text>"
    svg += text(runName, x: 32, y: 50, size: 11)
    svg += "<g transform=\"translate(0,64)\">"
    for id in layout.order {
      let node = tree.nodes[id]
      let p = point(id)!
      if let parent = node.parent, let q = point(parent) {
        let color = highlighted.contains(id) ? paper.highlight.css : paper.branch.css
        let width = highlighted.contains(id) ? paper.branchWidth + 0.6 : paper.branchWidth
        svg +=
          "<path d=\"M \(q.x) \(q.y) V \(p.y) H \(p.x)\" stroke=\"\(color)\" stroke-width=\"\(width)\" fill=\"none\"/>"
      }
    }
    for id in layout.order {
      let node = tree.nodes[id]
      let p = point(id)!
      svg += "<g><title>\(escape(label(id)))</title>"
      if collapsed.contains(id) && node.children != nil {
        svg +=
          "<path d=\"M \(p.x) \(p.y) L \(tipX) \(p.y-8) V \(p.y+8) Z\" fill=\"\(paper.highlight.css)\" fill-opacity=\"0.18\" stroke=\"\(paper.highlight.css)\"/>"
      }
      if node.constrained {
        svg +=
          "<path d=\"M \(p.x) \(p.y-5) L \(p.x+5) \(p.y) L \(p.x) \(p.y+5) L \(p.x-5) \(p.y) Z\" fill=\"\(paper.constrained.css)\"/>"
      } else {
        svg +=
          "<circle cx=\"\(p.x)\" cy=\"\(p.y)\" r=\"\(id == selected ? 4 : 2.5)\" fill=\"\(paper.node.css)\"/>"
      }
      if node.taxon != nil || collapsed.contains(id) {
        let q = labelPoint(id)
        svg += text(visibleLabel(id), x: q.x, y: q.y + 11)
      }
      if let support = support(id) {
        let q = supportPoint(id)
        svg += text(support, x: q.x, y: q.y + 11, size: 10, color: paper.support.css, halo: true)
      }
      svg += "</g>"
    }
    let sy = Double(size.height) - 35
    if scale.value > 0 {
      svg +=
        "<path d=\"M 32 \(sy-4) V \(sy) H \(32+scale.width) V \(sy-4)\" stroke=\"\(paper.scale.css)\" stroke-width=\"1.5\" fill=\"none\"/>"
    }
    svg += text(scaleLabel, x: 32, y: sy + 18, size: 10)
    svg += "</g>"
    let burnin = tree.summary.burnin
    let burn =
      burnin.unit == "percent"
      ? "\(burnin.value)%" : strings("%@ samples", strings.number(burnin.value))
    let policy = strings(
      tree.summary.heightPolicy == "ca" ? "Common ancestor mean" : "Keep selected tree")
    svg += text(
      strings(
        "Burn-in %@ · %@ retained · %@ discarded", burn,
        strings.number(tree.summary.retainedSamples), strings.number(tree.summary.discardedSamples)),
      x: 32, y: height - 65, size: 11)
    svg += text(
      policy + " · " + strings("Rates are separate multipliers."), x: 32, y: height - 47, size: 11)
    svg += text(
      strings("Support: retained clade frequency. Diamonds: constrained groups."), x: 32,
      y: height - 29, size: 11)
    svg += text(
      strings("A summary does not establish convergence."), x: 32, y: height - 11, size: 11)
    return svg + "</g></svg>\n"
  }
}
