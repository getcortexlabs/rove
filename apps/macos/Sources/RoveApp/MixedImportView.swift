import AppKit
import SwiftUI

struct MixedInputRequest: Sendable {
  let morphology: URL
  let dna: URL
  let output: URL
  let missingTaxa: String

  func execute(executable: URL) async throws {
    guard ["reject", "missing"].contains(missingTaxa) else { throw RunFailure.invalidCompletion }
    let outcome = try await EngineTool().execute(
      executable: executable,
      arguments: [
        "combine", "--morphology", morphology.path, "--dna", dna.path,
        "--output", output.path, "--missing-taxa", missingTaxa,
      ], onLine: { _ in })
    guard outcome.status == 0 else { throw ImportError.readerFailed(outcome.log) }
  }
}

struct MixedImportView: View {
  @Bindable var workspace: Workspace
  @State private var morphology: URL?
  @State private var dna: URL?
  @State private var missingTaxa = "reject"
  @State private var working = false
  @State private var error: String?
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    VStack(alignment: .leading, spacing: 18) {
      Text(strings("Combine morphology and DNA")).font(Lab.display(25))
      Text(
        strings(
          "Select one morphological NEXUS matrix and one aligned DNA file. Rove matches normalized taxon names and preserves both source files in a new combined input."
        )
      )
      .font(.system(size: 12)).foregroundStyle(Lab.secondary).fixedSize(
        horizontal: false, vertical: true)
      Button {
        morphology = choose("Choose morphology")
      } label: {
        Label(
          morphology?.lastPathComponent ?? strings("Choose morphology"),
          systemImage: "square.grid.3x3")
      }
      Button {
        dna = choose("Choose aligned DNA")
      } label: {
        Label(
          dna?.lastPathComponent ?? strings("Choose aligned DNA"), systemImage: "line.3.horizontal")
      }
      Picker(strings("Taxa absent from one source"), selection: $missingTaxa) {
        Text(strings("Require the same taxa")).tag("reject")
        Text(strings("Keep the union; absent cells are missing")).tag("missing")
      }
      Text(
        strings(
          "At least two taxa must be shared. Review unmatched names before choosing missing data; spelling differences are not biological absences."
        )
      )
      .font(.system(size: 11)).foregroundStyle(Lab.secondary).fixedSize(
        horizontal: false, vertical: true)
      if let error { Text(error).font(.system(size: 11)).textSelection(.enabled) }
      HStack {
        Button(strings("Cancel")) { workspace.showingMixedImport = false }.disabled(working)
        Spacer()
        Button(strings("Save combined input"), action: save).disabled(
          working || morphology == nil || dna == nil)
      }
    }
    .padding(28).frame(width: 610).foregroundStyle(Lab.ink).background(Lab.background)
    .disabled(working)
  }

  private func choose(_ title: String) -> URL? {
    let panel = NSOpenPanel()
    panel.title = strings(title)
    panel.canChooseDirectories = false
    panel.allowsMultipleSelection = false
    return panel.runModal() == .OK ? panel.url : nil
  }
  private func save() {
    guard let morphology, let dna else { return }
    let panel = NSSavePanel()
    panel.nameFieldStringValue = "analysis.mixed.json"
    panel.title = strings("Save combined input")
    guard panel.runModal() == .OK, let output = panel.url else { return }
    working = true
    error = nil
    Task {
      do {
        try await workspace.combineInput(
          MixedInputRequest(
            morphology: morphology, dna: dna, output: output, missingTaxa: missingTaxa))
        workspace.showingMixedImport = false
        workspace.load(output)
      } catch { self.error = error.localizedDescription }
      working = false
    }
  }
}
