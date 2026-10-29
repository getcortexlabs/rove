import Foundation
import Testing

@testable import RoveApp

struct TreeExplorerTests {
  static func balanced(tips: Int = 4) -> TreeDocument {
    var nodes: [DisplayTreeNode] = []
    func subtree(start: Int, count: Int, parent: Int?) {
      let id = nodes.count + 2 * count - 2
      if count > 1 {
        subtree(start: start, count: count / 2, parent: id)
        subtree(start: start + count / 2, count: count / 2, parent: id)
      }
      let height = log2(Double(count))
      nodes.append(
        DisplayTreeNode(
          id: id, parent: parent,
          children: count == 1 ? nil : [id - count, id - 1],
          taxon: count == 1 ? (start == 0 ? "A<&\"" : "T\(start)") : nil,
          height: height, length: parent == nil ? 0 : 1,
          posterior: count == 1 ? nil : (count == tips ? 1 : 2.0 / 3.0),
          constrained: count == 2 && start == 0, rate: nil,
          heightMean: nil, heightRange: nil, ancestorMean: nil, ancestorRange: nil, rateRange: nil))
    }
    subtree(start: 0, count: tips, parent: nil)
    return TreeDocument(
      schemaVersion: 1, kind: "posterior_summary", root: nodes.count - 1,
      nodes: nodes, taxa: nodes.compactMap(\.taxon), constraints: [["A<&\"", "T1"]],
      treeSha256: String(repeating: "0", count: 64),
      summary: TreeProvenance(
        method: "MCC", heightPolicy: "keep",
        burnin: .init(unit: "percent", value: 25), totalSamples: 4,
        discardedSamples: 1, retainedSamples: 3, selectedState: 100,
        sourcePath: "/example.trees", sourceSha256: String(repeating: "1", count: 64),
        sourceCompletion: "unverified", matrixSha256: nil, roveVersion: "0.1.0"))
  }

  @Test func layoutPreservesScientificDistancesAndDisplayChangesAreReversible() throws {
    let tree = Self.balanced()
    let original = try JSONEncoder().encode(tree)
    let decoded = try TreeDocument.decode(original)
    let layout = TreeLayout.build(decoded, collapsed: [], rotated: [])
    #expect(layout.rows == 4 && layout.positions[6]?.x == 0)
    #expect(layout.positions[2]?.x == 0.5 && layout.positions[0]?.x == 1)
    #expect(layout.positions[0]?.y == 0 && layout.positions[4]?.y == 3)
    let rotated = TreeLayout.build(decoded, collapsed: [], rotated: [2])
    #expect(rotated.positions[0]?.y == 1 && rotated.positions[1]?.y == 0)
    for id in tree.nodes.indices { #expect(rotated.positions[id]?.x == layout.positions[id]?.x) }
    let collapsed = TreeLayout.build(decoded, collapsed: [2], rotated: [2])
    #expect(collapsed.rows == 3 && collapsed.positions[0] == nil && collapsed.positions[1] == nil)
    #expect(collapsed.tipCounts[2] == 2 && collapsed.positions[2]?.x == 0.5)
    let restored = TreeLayout.build(decoded, collapsed: [], rotated: [])
    #expect(restored.order == layout.order)
    #expect(decoded.nodes.map(\.length) == tree.nodes.map(\.length))
    #expect(decoded.nodes.map(\.posterior) == tree.nodes.map(\.posterior))
    #expect(decoded.nodes.map(\.children) == tree.nodes.map(\.children))
  }

  @Test func malformedTopologyIsRejectedBeforeLayout() throws {
    let data = try JSONEncoder().encode(Self.balanced())
    var object = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
    var nodes = try #require(object["nodes"] as? [[String: Any]])
    nodes[2]["children"] = [2, 1]
    object["nodes"] = nodes
    let bad = try JSONSerialization.data(withJSONObject: object)
    #expect(throws: (any Error).self) { try TreeDocument.decode(bad) }
  }

  @Test func svgPreservesDisplayAndCannotReplaceProtectedFiles() throws {
    let tree = Self.balanced()
    let layout = TreeLayout.build(tree, collapsed: [2], rotated: [6])
    let figure = TreeFigure(
      tree: tree, layout: layout, viewport: CGSize(width: 600, height: 450),
      collapsed: [2], selected: 2, showSupport: false, strings: AppStrings(language: .english))
    let svg = figure.svg(runName: "Run <one> & two")
    #expect(XMLParser(data: Data(svg.utf8)).parse())
    #expect(svg.contains("Run &lt;one&gt; &amp; two"))
    #expect(svg.contains("A&lt;&amp;&quot;") && svg.contains("2 taxa"))
    #expect(svg.contains("25%") && svg.contains("3 retained") && svg.contains("source time units"))
    #expect(svg.contains("#B65C7C") && !svg.contains("0.6667</text>"))
    let x = Double(try #require(figure.point(2)).x)
    #expect(abs(x - (32 + figure.branchWidth / 2)) < 1e-10)
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-svg-\(UUID())")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: directory) }
    let source = directory.appendingPathComponent("source.svg")
    try Data("scientific input".utf8).write(to: source)
    let alias = directory.appendingPathComponent("alias.svg")
    try FileManager.default.createSymbolicLink(at: alias, withDestinationURL: source)
    for url in [source, alias, directory.appendingPathComponent("mcc.tree")] {
      #expect(throws: (any Error).self) {
        try figure.write(to: url, runName: "Test", protectedFiles: [source])
      }
    }
    #expect(try String(contentsOf: source, encoding: .utf8) == "scientific input")
    let output = directory.appendingPathComponent("figure.svg")
    try figure.write(to: output, runName: "Test", protectedFiles: [source])
    #expect(XMLParser(data: try Data(contentsOf: output)).parse())
  }

  @Test func largeLayoutAndExportRemainBounded() async throws {
    let tree = Self.balanced(tips: 4096)
    let bytes = try JSONEncoder().encode(tree)
    let start = ContinuousClock.now
    let (nodes, rows, svgSize) = try await Task.detached {
      let decoded = try TreeDocument.decode(bytes)
      let layout = TreeLayout.build(decoded, collapsed: [], rotated: [decoded.root])
      let figure = TreeFigure(
        tree: decoded, layout: layout, viewport: CGSize(width: 760, height: 600),
        collapsed: [], selected: nil, showSupport: true, strings: AppStrings(language: .english))
      let svg = figure.svg(runName: "4096-tip test")
      return (layout.order.count, layout.rows, svg.utf8.count)
    }.value
    #expect(nodes == 8191 && rows == 4096)
    #expect(svgSize < 8 * 1024 * 1024)
    print(
      "Tree explorer: decode, layout and SVG for 4096 tips: \(start.duration(to: .now)); SVG \(svgSize) bytes"
    )
  }
}
