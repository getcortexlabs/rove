import Foundation
import Testing

@testable import RoveApp

struct PreferencesTests {
  @Test func systemLanguageFallsBackToASupportedTranslation() {
    #expect(AppLanguage.system.resolved(preferredLanguages: ["pt-PT", "en-US"]) == .portuguese)
    #expect(AppLanguage.system.resolved(preferredLanguages: ["en-GB", "pt-BR"]) == .english)
    #expect(AppLanguage.system.resolved(preferredLanguages: ["fr-FR", "pt-BR"]) == .portuguese)
    #expect(AppLanguage.system.resolved(preferredLanguages: ["ja-JP"]) == .english)
    #expect(AppLanguage.english.resolved(preferredLanguages: ["pt-BR"]) == .english)
  }

  @Test @MainActor func preferencesPersistAndLoadTheSelectedBundle() throws {
    let suite = "rove-preferences-test-\(UUID().uuidString)"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    #expect(preferences.language == .system)
    #expect(preferences.strings("Settings") == "Settings")
    preferences.language = .portuguese
    preferences.reduceMotion = true
    preferences.reduceTransparency = true
    let restored = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    #expect(restored.language == .portuguese)
    #expect(restored.reduceMotion && restored.reduceTransparency)
    #expect(restored.strings("Settings") == "Configurações")
    #expect(
      restored.strings("Every %@ steps", restored.strings.number(10_000)) == "A cada 10.000 passos")
    #expect(
      restored.strings.matrixSummary(taxa: 1, characters: 1, patterns: 1)
        == "1 terminal · 1 caráter · 1 padrão")
    #expect(restored.strings.guideURL.path == "/pt-BR/guide")
    restored.language = .english
    #expect(
      restored.strings("Every %@ steps", restored.strings.number(10_000)) == "Every 10,000 steps")
  }

  @Test @MainActor func changingLanguagePreservesTheAnalysisAndExport() async throws {
    let suite = "rove-language-test-\(UUID().uuidString)"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let preferences = AppPreferences(defaults: defaults, preferredLanguages: ["en-US"])
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent()
      .deletingLastPathComponent().deletingLastPathComponent()
    let executable = root.appendingPathComponent("target/release/rove")
    let matrix = root.appendingPathComponent("fixtures/reference/20x50/matrix.nex")
    let report = try await MatrixInspector.inspect(
      matrix, gapsAsMissing: false, executable: executable)
    let workspace = Workspace(executable: executable, preferences: preferences)
    workspace.file = matrix
    workspace.report = report
    workspace.page = .review
    workspace.settings.seed = "123456"
    workspace.settings.steps = 100_000_000
    let settings = workspace.settings
    let output = root.appendingPathComponent("unused-test-output")
    let arguments = try settings.arguments(matrix: matrix, output: output)
    preferences.language = .portuguese
    #expect(workspace.page == .review)
    #expect(workspace.report?.inputSha256 == report.inputSha256)
    #expect(workspace.settings == settings)
    #expect(try workspace.settings.arguments(matrix: matrix, output: output) == arguments)
    #expect(workspace.canOpen(.review))
    workspace.settings.seed = "-1"
    let issue = try #require(workspace.issue)
    #expect(workspace.strings(issue).hasPrefix("Use uma semente inteira"))
    #expect(!workspace.canOpen(.review))
    preferences.language = .english
    #expect(workspace.strings(issue).hasPrefix("Use a whole-number seed"))
    #expect(workspace.settings.seed == "-1")
    #expect(workspace.report?.cells == report.cells)
  }
}
