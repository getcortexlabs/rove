import Foundation

struct ProcessRateInput: Codable, Equatable, Sendable {
  var estimated = false
  var mean = ""
}

struct ProcessEstimation: Codable, Equatable, Sendable {
  var birth = ProcessRateInput()
  var death = ProcessRateInput()
  var fossilSampling = ProcessRateInput()
  var origin = ProcessRateInput()
  var estimateSampling = false
  var samplingAlpha = ""
  var samplingBeta = ""
  var hasEstimated: Bool {
    birth.estimated || death.estimated || fossilSampling.estimated || origin.estimated
      || estimateSampling
  }

  var validationError: String? {
    for prior in [birth, death, fossilSampling, origin] where prior.estimated {
      guard let mean = DatingNumber.parse(prior.mean), mean > 0 else {
        return "Every estimated rate and origin excess needs a positive exponential prior mean."
      }
    }
    if estimateSampling {
      guard let a = DatingNumber.parse(samplingAlpha), let b = DatingNumber.parse(samplingBeta),
        (0.001...1000).contains(a), (0.001...1000).contains(b)
      else {
        return "Enter beta prior shapes from 0.001 to 1,000 for present-day sampling."
      }
    }
    return nil
  }

  func model() -> EngineProcessPriors? {
    guard hasEstimated else { return nil }
    return EngineProcessPriors(
      birthRateMean: birth.estimated ? DatingNumber.parse(birth.mean) : nil,
      deathRateMean: death.estimated ? DatingNumber.parse(death.mean) : nil,
      fossilSamplingRateMean: fossilSampling.estimated
        ? DatingNumber.parse(fossilSampling.mean) : nil,
      presentSamplingBeta: estimateSampling
        ? [DatingNumber.parse(samplingAlpha)!, DatingNumber.parse(samplingBeta)!] : nil,
      originExcessMean: origin.estimated ? DatingNumber.parse(origin.mean) : nil)
  }
}

struct EngineProcessPriors: Codable, Equatable, Sendable {
  let birthRateMean: Double?
  let deathRateMean: Double?
  let fossilSamplingRateMean: Double?
  let presentSamplingBeta: [Double]?
  let originExcessMean: Double?
}
