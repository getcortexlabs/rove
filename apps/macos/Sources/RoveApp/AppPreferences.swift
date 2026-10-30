import Foundation
import Observation

enum AppLanguage: String, CaseIterable, Identifiable, Sendable {
  case system
  case english = "en"
  case portuguese = "pt-BR"

  var id: String { rawValue }

  func resolved(preferredLanguages: [String] = Locale.preferredLanguages) -> Self {
    guard self == .system else { return self }
    for identifier in preferredLanguages {
      switch Locale(identifier: identifier).language.languageCode?.identifier {
      case "pt": return .portuguese
      case "en": return .english
      default: continue
      }
    }
    return .english
  }
}

@MainActor @Observable
final class AppPreferences {
  private let defaults: UserDefaults
  private let preferredLanguages: [String]

  var language: AppLanguage {
    didSet { defaults.set(language.rawValue, forKey: "interfaceLanguage") }
  }
  var reduceMotion: Bool {
    didSet { defaults.set(reduceMotion, forKey: "reduceMotion") }
  }
  var reduceTransparency: Bool {
    didSet { defaults.set(reduceTransparency, forKey: "reduceTransparency") }
  }

  init(defaults: UserDefaults = .standard, preferredLanguages: [String] = Locale.preferredLanguages)
  {
    self.defaults = defaults
    self.preferredLanguages = preferredLanguages
    language = AppLanguage(rawValue: defaults.string(forKey: "interfaceLanguage") ?? "") ?? .system
    reduceMotion = defaults.bool(forKey: "reduceMotion")
    reduceTransparency = defaults.bool(forKey: "reduceTransparency")
  }

  var strings: AppStrings {
    AppStrings(language: language.resolved(preferredLanguages: preferredLanguages))
  }
}

struct AppStrings: Sendable {
  let language: AppLanguage
  var locale: Locale { Locale(identifier: language == .portuguese ? "pt_BR" : "en_US") }
  var guideURL: URL {
    URL(
      string: language == .portuguese
        ? "https://rove.getcortexlabs.com/pt-BR/guide/"
        : "https://rove.getcortexlabs.com/guide/")!
  }

  private static let bundles: [AppLanguage: Bundle] = Dictionary(
    uniqueKeysWithValues: [AppLanguage.english, .portuguese].compactMap { language in
      guard let url = Lab.resource(language.rawValue, extension: "lproj"),
        let bundle = Bundle(url: url)
      else { return nil }
      return (language, bundle)
    })

  func callAsFunction(_ key: String, _ arguments: CVarArg...) -> String {
    let text = Self.bundles[language]?.localizedString(forKey: key, value: key, table: nil) ?? key
    return arguments.isEmpty ? text : String(format: text, locale: locale, arguments: arguments)
  }

  func number(_ value: Int) -> String { value.formatted(.number.locale(locale)) }

  func partitionSummary(states: Int, characters: Int) -> String {
    if characters == 1 { return self("%@ states · 1 character", number(states)) }
    return self("%@ states · %@ characters", number(states), number(characters))
  }

  func partitionSummary(_ partition: CharacterPartition) -> String {
    partitionSummary(states: partition.states, characters: partition.characters)
      + " · " + self(partition.ordering == .ordered ? "Ordered" : "Unordered")
  }

  func matrixSummary(taxa: Int, characters: Int, patterns: Int? = nil) -> String {
    var parts = [
      self(taxa == 1 ? "%@ terminal" : "%@ terminals", number(taxa)),
      self(characters == 1 ? "%@ character" : "%@ characters", number(characters)),
    ]
    if let patterns {
      parts.append(self(patterns == 1 ? "%@ pattern" : "%@ patterns", number(patterns)))
    }
    return parts.joined(separator: " · ")
  }
}
