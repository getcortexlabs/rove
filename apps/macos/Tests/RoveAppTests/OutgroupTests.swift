import Foundation
import Testing

@testable import RoveApp

struct OutgroupTests {
  private var root: URL {
    URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
  }
  private var engine: URL {
    ProcessInfo.processInfo.environment["ROVE_TEST_ENGINE"].map { URL(fileURLWithPath: $0) }
      ?? root.appendingPathComponent("target/release/rove")
  }
  private var matrix: URL { root.appendingPathComponent("fixtures/reference/20x50/matrix.nex") }

  @Test @MainActor func everySavedTreeKeepsTheOutgroupAndAllTerminals() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let suite = "rove-outgroup-\(UUID())"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer {
      defaults.removePersistentDomain(forName: suite)
      try? FileManager.default.removeItem(at: directory)
    }
    let workspace = Workspace(
      executable: engine, preferences: AppPreferences(defaults: defaults),
      history: RunHistory(defaults: defaults, defaultRoot: directory))
    workspace.load(matrix)
    try await finishImport(workspace)
    #expect(workspace.outgroup == nil && workspace.constraints.isEmpty)
    workspace.outputParent = directory
    for clock in ["strict", "reference"] {
      workspace.outgroup = "T20"
      workspace.settings.clock = clock
      workspace.settings.steps = 10_000
      workspace.settings.sampleEvery = 100
      workspace.page = .review
      workspace.startRun()
      let run = try #require(workspace.run)
      #expect(!workspace.applyGroup(GroupSelection(group: ["T1", "T2"])))
      await run.waitForExit()
      #expect(run.status == .completed)
      let record = try EngineRecord.read(run.request.output.appendingPathComponent("run.json"))
      #expect(record.matches(run.request.draft))
      #expect(record.model.monophyly == [(1...19).map { "T\($0)" }])
      let history = HistoryEntry.read(run.request.directory)
      #expect(history.status == .completed && history.draft?.outgroup == "T20")
      #expect(history.draft?.taxa?.count == 20)
      let trees = try String(
        contentsOf: run.request.output.appendingPathComponent("trees.nexus"), encoding: .utf8
      )
      .split(separator: "\n").filter { $0.hasPrefix("tree STATE_") }
      #expect(trees.count == 101)
      #expect(trees.first?.hasPrefix("tree STATE_0 = ") == true)
      for line in trees {
        let newick = try #require(line.split(separator: " = ").last)
        let (left, right) = try rootChildren(String(newick))
        let sides = [try tips(left), try tips(right)]
        #expect(sides.contains([20]))
        #expect(sides.contains(Set(1...19)))
        #expect(sides[0].isDisjoint(with: sides[1]))
      }
      workspace.dismissRun()
    }
  }

  @Test @MainActor func groupEditsStayPendingUntilAppliedAndRejectRootConflicts() async throws {
    let workspace = Workspace(executable: engine)
    workspace.load(matrix)
    try await finishImport(workspace)
    let taxa = try #require(workspace.report?.allTaxa)
    var selection = GroupSelection(group: taxa)
    selection.selected.remove("T20")
    #expect(workspace.monophyly.isEmpty)
    #expect(workspace.applyGroup(selection))
    let ingroup = Array(taxa.dropLast())
    #expect(workspace.monophyly == [ingroup])
    // A legacy-style group is not an explicit outgroup choice.
    #expect(workspace.outgroup == nil)
    var cancelled = GroupSelection(index: 0, group: workspace.monophyly[0])
    cancelled.selected.remove("T19")
    // Dismissing this value leaves the applied group alone.
    #expect(workspace.monophyly == [ingroup])
    let reopened = GroupSelection(index: 0, group: workspace.monophyly[0])
    #expect(reopened.selected == Set(ingroup))
    workspace.outgroup = "T20"
    // No duplicate derived group.
    #expect(workspace.constraints == [ingroup])
    #expect(workspace.applyGroup(cancelled))
    #expect(workspace.constraints == [ingroup, Array(ingroup.dropLast())])
    let previous = workspace.monophyly
    #expect(!workspace.applyGroup(GroupSelection(group: ["T1", "T20"])))
    #expect(!workspace.applyGroup(GroupSelection(index: 99, group: ["T1", "T2"])))
    #expect(workspace.monophyly == previous)
    workspace.outgroup = "unknown"
    #expect(workspace.issue != nil && !workspace.canStartRun)
    workspace.outgroup = nil
    // Removing the outgroup preserves manual groups.
    #expect(workspace.constraints == previous)
    workspace.load(matrix)
    try await finishImport(workspace)
    #expect(workspace.outgroup == nil && workspace.constraints.isEmpty)
    workspace.outgroup = "T20"
    workspace.reloadGapPolicy()
    try await finishImport(workspace)
    #expect(workspace.outgroup == "T20")
    workspace.reset()
    #expect(workspace.outgroup == nil && workspace.constraints.isEmpty)
  }

  @Test @MainActor func setupsAndHistoryPreserveRootingAndReadLegacyGroups() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let report = try await MatrixInspector.inspect(matrix, gapsAsMissing: false, executable: engine)
    let draft = AnalysisDraft(
      matrix: matrix, report: report, settings: AnalysisSettings(),
      monophyly: [["T1", "T2"]], outgroup: "T20")
    let path = directory.appendingPathComponent("setup.rove.json")
    try draft.write(to: path)
    let saved = try AnalysisDraft.read(path)
    #expect(saved.outgroup == "T20" && saved.taxa == report.allTaxa)
    #expect(saved.appliedMonophyly == saved.constraints && saved.constraints.count == 2)
    let workspace = Workspace(executable: engine)
    workspace.loadSetup(path)
    try await finishImport(workspace)
    #expect(workspace.error == nil && workspace.canStartRun)
    #expect(workspace.outgroup == "T20" && workspace.monophyly == [["T1", "T2"]])
    #expect(workspace.constraints == saved.constraints)
    workspace.reset()
    // A folder with a valid saved setup but no session is still reusable from History.
    workspace.reuseSetup(HistoryEntry.read(directory))
    try await finishImport(workspace)
    #expect(workspace.outgroup == "T20" && workspace.run == nil)
    #expect(workspace.constraints == saved.constraints)

    let original = try Data(contentsOf: path)
    var json = try #require(JSONSerialization.jsonObject(with: original) as? [String: Any])
    json["formatVersion"] = 1
    json["monophyly"] = [Array(report.allTaxa.dropLast())]
    for field in ["outgroup", "taxa", "appliedMonophyly"] { json.removeValue(forKey: field) }
    try JSONSerialization.data(withJSONObject: json).write(to: path)
    workspace.loadSetup(path)
    try await finishImport(workspace)
    #expect(workspace.error == nil && workspace.outgroup == nil)
    #expect(workspace.monophyly == [Array(report.allTaxa.dropLast())])
    #expect(try AnalysisDraft.read(path).taxa == nil)

    json = try #require(JSONSerialization.jsonObject(with: original) as? [String: Any])
    json["appliedMonophyly"] = [["T1", "T2"]]
    try JSONSerialization.data(withJSONObject: json).write(to: path)
    #expect(throws: (any Error).self) { try AnalysisDraft.read(path) }
    json = try #require(JSONSerialization.jsonObject(with: original) as? [String: Any])
    json["taxa"] = report.allTaxa + ["unknown"]
    json["appliedMonophyly"] = [Array(report.allTaxa.dropLast()) + ["unknown"], ["T1", "T2"]]
    try JSONSerialization.data(withJSONObject: json).write(to: path)
    workspace.loadSetup(path)
    try await finishImport(workspace)
    #expect(workspace.error == "The saved terminals do not match the original matrix.")
    #expect(workspace.outgroup == nil && workspace.monophyly == [Array(report.allTaxa.dropLast())])
  }

  @Test func rootingValidationRejectsConflictsAndDoesNotInventAnOutgroup() {
    let taxa = ["A", "B", "C", "D"]
    #expect(GroupConstraints.validate([], taxa: taxa, outgroup: "D") == nil)
    #expect(GroupConstraints.applied([], taxa: taxa, outgroup: nil).isEmpty)
    #expect(GroupConstraints.validate([["A", "B"]], taxa: taxa, outgroup: "D") == nil)
    #expect(GroupConstraints.validate([["A", "D"]], taxa: taxa, outgroup: "D") != nil)
    #expect(GroupConstraints.validate([], taxa: taxa, outgroup: "unknown") != nil)
    #expect(GroupConstraints.validate([], taxa: ["A", "B"], outgroup: "B") != nil)
    #expect(
      GroupConstraints.validate([["A", "B"], ["B", "A"]], taxa: taxa, outgroup: "D") != nil)
  }

  // Only the generated rooted Newick's outer split is needed for this integration check.
  private func rootChildren(_ newick: String) throws -> (String, String) {
    let characters = Array(newick)
    var depth = 0
    var annotation = false
    for index in characters.indices {
      let char = characters[index]
      if char == "[" { annotation = true }
      if char == "]" { annotation = false }
      if annotation { continue }
      if char == "(" { depth += 1 }
      if char == ")" { depth -= 1 }
      if char == "," && depth == 1 {
        return (
          String(characters[1..<index]), String(characters[(index + 1)..<(characters.count - 2)])
        )
      }
    }
    throw ImportError.message("Missing root split in generated tree.")
  }

  private func tips(_ branch: String) throws -> Set<Int> {
    let expression = try NSRegularExpression(pattern: #"(?:^|[(,])(\d+)(?=\[|:)"#)
    let text = branch as NSString
    return Set(
      expression.matches(in: branch, range: NSRange(location: 0, length: text.length))
        .compactMap { Int(text.substring(with: $0.range(at: 1))) })
  }

  @MainActor private func finishImport(_ workspace: Workspace) async throws {
    for _ in 0..<100 {
      if !workspace.loading { return }
      try await Task.sleep(for: .milliseconds(50))
    }
    throw ImportError.message("Setup import did not finish.")
  }
}
