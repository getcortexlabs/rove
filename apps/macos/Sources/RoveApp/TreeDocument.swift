import Foundation

struct DisplayTreeNode: Codable, Sendable, Identifiable {
  let id: Int
  let parent: Int?
  let children: [Int]?
  let taxon: String?
  let height: Double
  let length: Double
  let posterior: Double?
  let constrained: Bool
  let rate: Double?
  let heightMean: Double?
  let heightRange: [Double]?
  let ancestorMean: Double?
  let ancestorRange: [Double]?
  let rateRange: [Double]?
}

struct TreeProvenance: Codable, Sendable {
  struct Burnin: Codable, Sendable {
    let unit: String
    let value: Int
  }
  let method: String
  let heightPolicy: String
  let burnin: Burnin
  let totalSamples: Int
  let discardedSamples: Int
  let retainedSamples: Int
  let selectedState: UInt64
  let sourcePath: String
  let sourceSha256: String
  let sourceCompletion: String
  let matrixSha256: String?
  let roveVersion: String
}

struct TreeDocument: Codable, Sendable {
  let schemaVersion: Int
  let kind: String
  let root: Int
  let nodes: [DisplayTreeNode]
  let taxa: [String]
  let constraints: [[String]]
  let treeSha256: String
  let summary: TreeProvenance

  static func decode(_ data: Data) throws -> Self {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    let tree = try decoder.decode(Self.self, from: data)
    guard tree.schemaVersion == 1, tree.kind == "posterior_summary", tree.summary.method == "MCC",
      tree.nodes.count == tree.taxa.count * 2 - 1, (2...4096).contains(tree.taxa.count),
      tree.root == tree.nodes.count - 1, tree.nodes[tree.root].parent == nil
    else { throw ImportError.message("The annotated tree is invalid.") }
    var parents = [Int](repeating: 0, count: tree.nodes.count)
    var names = Set<String>()
    for (id, node) in tree.nodes.enumerated() {
      guard node.id == id, node.height.isFinite, node.height >= 0, node.length.isFinite,
        node.length >= 0, node.posterior.map({ $0.isFinite && (0...1).contains($0) }) ?? true
      else { throw ImportError.message("The annotated tree is invalid.") }
      if let children = node.children {
        guard children.count == 2, children[0] != children[1], node.taxon == nil else {
          throw ImportError.message("The annotated tree is invalid.")
        }
        for child in children {
          guard child >= 0, child < id, tree.nodes[child].parent == id,
            tree.nodes[child].height <= node.height + 1e-12
          else { throw ImportError.message("The annotated tree is invalid.") }
          parents[child] += 1
        }
      } else {
        guard let name = node.taxon, !name.isEmpty, names.insert(name).inserted else {
          throw ImportError.message("The annotated tree is invalid.")
        }
      }
      for range in [node.heightRange, node.ancestorRange, node.rateRange].compactMap({ $0 }) {
        guard range.count == 2, range.allSatisfy({ $0.isFinite && $0 >= 0 }), range[0] <= range[1]
        else {
          throw ImportError.message("The annotated tree is invalid.")
        }
      }
    }
    guard parents.dropLast().allSatisfy({ $0 == 1 }), names == Set(tree.taxa) else {
      throw ImportError.message("The annotated tree is invalid.")
    }
    return tree
  }

  func descendants(of id: Int) -> [Int] {
    var stack = [id]
    var result: [Int] = []
    while let next = stack.popLast() {
      result.append(next)
      stack.append(contentsOf: nodes[next].children ?? [])
    }
    return result
  }
}

struct TreeLayout: Sendable {
  struct Point: Sendable {
    let x: Double
    var y: Double
  }
  let positions: [Point?]
  let order: [Int]
  let tipCounts: [Int]
  let rows: Int
  let collapsed: Set<Int>
  let maxLabelCharacters: Int

  static func build(_ tree: TreeDocument, collapsed: Set<Int>, rotated: Set<Int>) -> Self {
    var positions = [Point?](repeating: nil, count: tree.nodes.count)
    var counts = [Int](repeating: 1, count: tree.nodes.count)
    for node in tree.nodes {
      if let children = node.children { counts[node.id] = children.reduce(0) { $0 + counts[$1] } }
    }
    var stack = [tree.root]
    var visited: [Int] = []
    var rows = 0
    let height = tree.nodes[tree.root].height
    while let id = stack.popLast() {
      let node = tree.nodes[id]
      let x = height > 0 ? (height - node.height) / height : 0
      visited.append(id)
      if let children = node.children, !collapsed.contains(id) {
        let order = rotated.contains(id) ? Array(children.reversed()) : children
        stack.append(contentsOf: order.reversed())
        positions[id] = Point(x: x, y: 0)
      } else {
        positions[id] = Point(x: x, y: Double(rows))
        rows += 1
      }
    }
    for id in visited.reversed() {
      if let children = tree.nodes[id].children, !collapsed.contains(id) {
        let y = (positions[children[0]]!.y + positions[children[1]]!.y) / 2
        positions[id]?.y = y
      }
    }
    let order = visited.sorted {
      let a = positions[$0]!
      let b = positions[$1]!
      return a.y == b.y ? (a.x == b.x ? $0 < $1 : a.x < b.x) : a.y < b.y
    }
    return Self(
      positions: positions, order: order, tipCounts: counts, rows: rows, collapsed: collapsed,
      maxLabelCharacters: tree.taxa.map { min($0.count, 44) }.max() ?? 0)
  }
}

enum TreeInspector {
  static func read(directory: URL, executable: URL) throws -> TreeDocument {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(
      "rove-tree-\(UUID())")
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: folder) }
    let output = folder.appendingPathComponent("tree.json")
    let errors = folder.appendingPathComponent("errors.log")
    FileManager.default.createFile(atPath: output.path, contents: nil)
    FileManager.default.createFile(atPath: errors.path, contents: nil)
    let stdout = try FileHandle(forWritingTo: output)
    let stderr = try FileHandle(forWritingTo: errors)
    defer {
      try? stdout.close()
      try? stderr.close()
    }
    let process = Process()
    process.executableURL = executable
    process.arguments = ["inspect-tree", directory.path]
    process.standardOutput = stdout
    process.standardError = stderr
    process.standardInput = FileHandle.nullDevice
    try process.run()
    process.waitUntilExit()
    guard process.terminationStatus == 0 else {
      let log = try FileHandle(forReadingFrom: errors)
      defer { try? log.close() }
      throw ImportError.readerFailed(
        String(decoding: try log.read(upToCount: 65536) ?? Data(), as: UTF8.self))
    }
    let input = try FileHandle(forReadingFrom: output)
    defer { try? input.close() }
    let bytes = try input.read(upToCount: 16 * 1024 * 1024 + 1) ?? Data()
    guard bytes.count <= 16 * 1024 * 1024 else {
      throw ImportError.message("The annotated tree is too large.")
    }
    return try TreeDocument.decode(bytes)
  }
}
