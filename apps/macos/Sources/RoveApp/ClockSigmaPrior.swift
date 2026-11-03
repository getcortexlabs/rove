import Charts
import SwiftUI

struct ClockSigmaPrior: Codable, Equatable, Sendable {
  let shape: Double
  let scale: Double
  static let reference = Self(shape: 5, scale: 0.2)

  func description(strings: AppStrings) -> String {
    strings("Gamma · shape %@ · scale %@", String(shape), String(scale))
  }

  var mean: Double { shape * scale }
  var standardDeviation: Double { sqrt(shape) * scale }

  /// Shape/scale Gamma density; evaluate in scaled coordinates to avoid
  /// subtracting the separate, large shape * log(scale) terms.
  func logDensity(at sigma: Double) -> Double? {
    guard shape.isFinite, scale.isFinite, shape > 0, scale > 0,
      sigma.isFinite, sigma >= 0, lgamma(shape).isFinite
    else { return nil }
    if sigma == 0 {
      if shape < 1 { return .infinity }
      return shape == 1 ? -log(scale) : -.infinity
    }
    let value = sigma / scale
    guard value.isFinite, value > 0 else { return nil }
    let density = (shape - 1) * log(value) - value - lgamma(shape) - log(scale)
    return density.isNaN ? nil : density
  }

  struct Curve: Sendable {
    struct Point: Identifiable, Sendable {
      let id: Int
      let sigma: Double
      let density: Double
    }
    let points: [Point]
    let upperBound: Double
  }

  var curve: Curve? {
    let upper = mean + 5 * standardDeviation
    // The numerical prior may be valid at scales where a linear display
    // cannot draw meaningful axes. Keep that limitation in the preview only.
    guard upper.isFinite, upper >= 1e-12, upper <= 1e12,
      mean.isFinite, mean > 0, shape > 0, scale > 0
    else { return nil }
    let first = shape < 1 ? 1 : 0
    var points: [Curve.Point] = []
    for index in first...240 {
      let sigma = upper * Double(index) / 240
      guard let logValue = logDensity(at: sigma) else { return nil }
      let value = exp(logValue)
      guard value.isFinite, value <= 1e12 else { return nil }
      points.append(.init(id: index, sigma: sigma, density: value))
    }
    guard points.contains(where: { $0.density > 0 }) else { return nil }
    return Curve(points: points, upperBound: upper)
  }
}

struct ClockSigmaPriorChart: View {
  let prior: ClockSigmaPrior
  let strings: AppStrings

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      if let curve = prior.curve {
        Chart {
          ForEach(curve.points) { point in
            LineMark(
              x: .value("Sigma", point.sigma),
              y: .value(strings("Prior density"), point.density)
            )
            .foregroundStyle(Lab.lilac).lineStyle(StrokeStyle(lineWidth: 2))
          }
          RuleMark(x: .value(strings("Expected sigma"), prior.mean))
            .foregroundStyle(Lab.accent).lineStyle(StrokeStyle(lineWidth: 1, dash: [4, 3]))
        }
        .chartXScale(domain: 0...curve.upperBound)
        .chartXAxisLabel("Sigma")
        .chartYAxisLabel(strings("Prior density"))
        .frame(height: 170)
        .accessibilityLabel(
          strings("Gamma prior for clock sigma; the dashed line marks its expectation."))
      } else {
        Text(strings("The density preview is unavailable at this numerical scale."))
          .font(.system(size: 11)).foregroundStyle(Lab.secondary)
      }
      if prior.mean.isFinite, prior.standardDeviation.isFinite {
        Text(
          strings(
            "Expected sigma: %@ · standard deviation: %@",
            number(prior.mean), number(prior.standardDeviation))
        )
        .font(.system(size: 11, weight: .medium)).foregroundStyle(Lab.accent)
      }
      if prior.shape < 1 {
        Text(strings("The density grows without bound near zero; the curve starts above zero."))
          .font(.system(size: 10)).foregroundStyle(Lab.secondary)
      }
      Text(
        strings(
          "This is a prior preview, before seeing the data. The displayed range is not a credible interval."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted)
    }
  }

  private func number(_ value: Double) -> String {
    value.formatted(.number.precision(.significantDigits(1...5)).locale(strings.locale))
  }
}

struct ClockSigmaPriorSection: View {
  @Bindable var workspace: Workspace
  @State private var expanded = false
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    LabPanel {
      DisclosureGroup(isExpanded: $expanded) {
        VStack(alignment: .leading, spacing: 14) {
          Text(
            strings(
              "Sigma controls how much branch rates can differ. This Gamma distribution expresses what you expect about sigma before looking at the characters."
            )
          )
          .font(.system(size: 12)).foregroundStyle(Lab.secondary)
          HStack(spacing: 18) {
            field("Shape · Alpha", text: $workspace.settings.clockSigmaShape)
            field("Scale · Beta", text: $workspace.settings.clockSigmaScale)
            Button(strings("Restore defaults")) {
              workspace.settings.clockSigmaShape = "5"
              workspace.settings.clockSigmaScale = "0.2"
            }.buttonStyle(.borderless).tint(Lab.accent)
          }
          Text(
            strings(
              "Beta is a scale, not a rate. The expected sigma is Alpha × Beta (1 with the defaults). These settings do not change gamma variation across characters and are not a shortcut to convergence."
            )
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
          if let prior = workspace.settings.clockSigmaPrior {
            ClockSigmaPriorChart(prior: prior, strings: strings)
          }
          if workspace.settings.clockSigmaPrior == nil {
            Text(strings("Use positive, finite shape and scale values for the clock sigma prior."))
              .font(.system(size: 12)).foregroundStyle(Lab.accent)
          }
        }.padding(.top, 14)
      } label: {
        HStack {
          Text(strings("Clock sigma prior")).font(.system(size: 13, weight: .semibold))
          Spacer()
          if let prior = workspace.settings.clockSigmaPrior {
            Text(prior.description(strings: strings)).font(.system(size: 11)).foregroundStyle(
              Lab.secondary)
          }
        }
      }.tint(Lab.accent)
        .onAppear { expanded = workspace.settings.usesCustomSigmaPrior }
    }
  }

  private func field(_ title: String, text: Binding<String>) -> some View {
    VStack(alignment: .leading, spacing: 6) {
      Text(strings(title)).font(.system(size: 11)).foregroundStyle(Lab.secondary)
      TextField(strings(title), text: text).textFieldStyle(.roundedBorder).frame(width: 140)
    }
  }
}
