import Foundation

// Keep editable text until validation, so an incomplete field cannot silently
// reuse an earlier valid number. Commas and dots are decimal separators, not grouping.
enum DatingNumber {
  static func parse(_ text: String) -> Double? {
    let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !text.contains(",") || !text.contains("."),
      let value = Double(text.replacingOccurrences(of: ",", with: ".")), value.isFinite
    else { return nil }
    return value
  }
}

struct TipAgeInput: Codable, Equatable, Sendable {
  enum Kind: String, Codable, CaseIterable, Sendable {
    case present, fixed, interval
    var title: String {
      switch self {
      case .present: "Present-day"
      case .fixed: "Fixed age"
      case .interval: "Age interval"
      }
    }
  }
  var kind = Kind.present
  var fixed = ""
  var minimum = ""
  var maximum = ""

  var age: EngineTipAge? {
    switch kind {
    case .present: return .fixed(0)
    case .fixed:
      guard let age = DatingNumber.parse(fixed), age > 0 else { return nil }
      return .fixed(age)
    case .interval:
      guard let lower = DatingNumber.parse(minimum), let upper = DatingNumber.parse(maximum),
        lower > 0, upper > lower
      else { return nil }
      return .interval(lower, upper)
    }
  }
}

struct DatingSettings: Codable, Equatable, Sendable {
  enum Clock: String, Codable, Sendable { case fixed, estimated }
  var enabled = false
  var timeUnit = "Ma"
  var birthRate = ""
  var deathRate = ""
  var fossilSamplingRate = ""
  var presentSampling = ""
  var origin = ""
  var clockRate = ""
  var clock = Clock.estimated
  var clockPriorMean = ""
  var ages: [String: TipAgeInput] = [:]
  var processEstimation: ProcessEstimation? = nil
  var estimation: ProcessEstimation {
    get { processEstimation ?? ProcessEstimation() }
    set { processEstimation = newValue }
  }
  var estimatesProcess: Bool { enabled && estimation.hasEstimated }

  var unit: String { timeUnit.trimmingCharacters(in: .whitespacesAndNewlines) }
  var fossilCount: Int { ages.values.filter { $0.kind != .present }.count }
  var intervalCount: Int { ages.values.filter { $0.kind == .interval }.count }
  var oldestAge: Double? { ages.values.compactMap { $0.age?.maximum }.max() }

  func validationError(taxa: [String]? = nil) -> String? {
    guard enabled else { return nil }
    guard !unit.isEmpty, unit.utf8.count <= 80,
      !unit.unicodeScalars.contains(where: CharacterSet.controlCharacters.contains)
    else { return "Enter a time unit, such as Ma for millions of years." }
    guard let birth = DatingNumber.parse(birthRate), birth > 0,
      let death = DatingNumber.parse(deathRate), death >= 0,
      let sampling = DatingNumber.parse(fossilSamplingRate), sampling > 0
    else { return "Enter positive birth and fossil sampling rates, and a nonnegative death rate." }
    guard let rho = DatingNumber.parse(presentSampling), (0...1).contains(rho) else {
      return "Present-day sampling must be a probability from 0 to 1."
    }
    if let issue = estimation.validationError { return issue }
    if estimation.death.estimated && death <= 0 {
      return "An estimated death rate needs a positive starting value."
    }
    if estimation.estimateSampling && !(rho > 0 && rho < 1) {
      return "Estimated present-day sampling must start strictly between 0 and 1."
    }
    guard let start = DatingNumber.parse(origin), start > 0 else {
      return "Enter a positive origin age in the selected time unit."
    }
    guard let rate = DatingNumber.parse(clockRate), rate > 0,
      clock != .estimated || (DatingNumber.parse(clockPriorMean).map { $0 > 0 } ?? false)
    else { return "Enter a positive clock rate and, when estimated, a positive prior mean." }
    if let error = Self.ageError(ages, taxa: taxa) { return error }
    guard ages.values.allSatisfy({ $0.age!.maximum < start }) else {
      return "The origin must be older than every allowed tip age, including interval maxima."
    }
    guard rho > 0 || ages.values.allSatisfy({ $0.kind != .present }) else {
      return "Present-day sampling must be positive when any terminal is present-day."
    }
    guard death > 0 || rho < 1 || fossilCount == 0 else {
      return
        "Terminal fossils require extinction or incomplete present-day sampling in this FBD model."
    }
    // Match the engine's representable FBD coefficients before launching it.
    let c = hypot(birth - death - sampling, 2 * sqrt(birth) * sqrt(sampling))
    let b = 1 + (2 * birth * rho - birth + death + sampling) / c
    guard c.isFinite, c > 0, b.isFinite, b > 0,
      (birth + death + sampling + c).isFinite, (c * start).isFinite
    else { return "These process values exceed the numerical range. Check their time units." }
    return nil
  }

  static func ageError(_ ages: [String: TipAgeInput], taxa: [String]? = nil) -> String? {
    guard !ages.isEmpty, taxa.map({ Set($0) == Set(ages.keys) }) ?? true else {
      return "Assign an age to every terminal in this matrix."
    }
    guard ages.values.allSatisfy({ $0.age != nil }) else {
      return "Use positive fossil ages. Intervals need a younger minimum and an older maximum."
    }
    return nil
  }

  func model() throws -> EngineDatingModel? {
    guard enabled else { return nil }
    if let error = validationError() { throw ImportError.message(error) }
    return EngineDatingModel(
      timeUnit: unit, birthRate: DatingNumber.parse(birthRate)!,
      deathRate: DatingNumber.parse(deathRate)!,
      fossilSamplingRate: DatingNumber.parse(fossilSamplingRate)!,
      presentSampling: DatingNumber.parse(presentSampling)!, origin: DatingNumber.parse(origin)!,
      clockRate: DatingNumber.parse(clockRate)!,
      clockPriorMean: clock == .estimated ? DatingNumber.parse(clockPriorMean) : nil,
      ages: ages.mapValues { $0.age! },
      processModel: estimatesProcess ? "joint_sampling_v1" : nil,
      processPriors: estimation.model())
  }
}

// The wire format mirrors the engine's dating JSON. Setup files retain the
// editable strings separately, including precision and the selected clock mode.
struct EngineDatingModel: Codable, Equatable, Sendable {
  let timeUnit: String
  let birthRate: Double
  let deathRate: Double
  let fossilSamplingRate: Double
  let presentSampling: Double
  let origin: Double
  let clockRate: Double
  let clockPriorMean: Double?
  let ages: [String: EngineTipAge]
  var processModel: String? = nil
  var processPriors: EngineProcessPriors? = nil

  func json() throws -> String {
    let encoder = JSONEncoder()
    encoder.keyEncodingStrategy = .convertToSnakeCase
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    return String(decoding: try encoder.encode(self), as: UTF8.self)
  }

  static func decode(_ json: String) throws -> Self {
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    return try decoder.decode(Self.self, from: Data(json.utf8))
  }
}

enum EngineTipAge: Codable, Equatable, Sendable {
  case fixed(Double)
  case interval(Double, Double)
  var maximum: Double {
    switch self {
    case .fixed(let value): value
    case .interval(_, let upper): upper
    }
  }
  init(from decoder: Decoder) throws {
    let value = try decoder.singleValueContainer()
    if let age = try? value.decode(Double.self) {
      self = .fixed(age)
    } else {
      let bounds = try value.decode([Double].self)
      guard bounds.count == 2 else {
        throw DecodingError.dataCorruptedError(
          in: value, debugDescription: "Expected two age bounds")
      }
      self = .interval(bounds[0], bounds[1])
    }
  }
  func encode(to encoder: Encoder) throws {
    var value = encoder.singleValueContainer()
    switch self {
    case .fixed(let age): try value.encode(age)
    case .interval(let lower, let upper): try value.encode([lower, upper])
    }
  }
}
