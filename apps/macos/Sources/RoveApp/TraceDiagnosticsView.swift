import Charts
import SwiftUI

struct TraceDiagnosticsView: View {
  @Bindable var model: TraceDiagnosticsModel
  let preferences: AppPreferences
  let onOpenTree: (URL) -> Void
  @State private var includeDiscarded = true
  @State private var summary: TreeSummaryModel?
  private var strings: AppStrings { preferences.strings }

  var body: some View {
    VStack(spacing: 0) {
      header.padding(22)
      HStack(spacing: 12) {
        Text(strings("Burn-in (%)")).font(.system(size: 12, weight: .medium))
        TextField(strings("Burn-in (%)"), text: $model.burnin).textFieldStyle(.roundedBorder)
          .frame(width: 58).disabled(model.busy).onSubmit { model.start() }
        Button(strings("Apply"), action: model.start)
          .buttonStyle(LabButtonStyle(compact: true)).disabled(!model.valid || model.busy)
        if model.busy {
          ProgressView().controlSize(.small)
          Text(strings("Calculating from the retained samples…")).font(.system(size: 11))
          Spacer()
          Button(strings("Cancel"), action: model.cancel)
        } else {
          Text(
            strings(
              "Choose burn-in after inspecting early trends. There is no universal percentage.")
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary)
          Spacer(minLength: 0)
        }
      }.padding(.horizontal, 22).padding(.bottom, 18)
      if let saved = model.result {
        HStack(spacing: 0) {
          parameterList.frame(width: 260)
          Divider()
          ScrollView {
            VStack(alignment: .leading, spacing: 18) {
              if saved.report.source.completion != "completed" {
                Label(
                  strings(
                    "This is a partial or unverified run. These numbers describe only the available samples."
                  ), systemImage: "exclamationmark.circle"
                )
                .foregroundStyle(Lab.coral).font(.system(size: 12))
              }
              if model.burnin != String(saved.report.burnin.value) || model.busy {
                Text(
                  strings(
                    "Displayed results use %@% burn-in. Apply your change to update them.",
                    String(saved.report.burnin.value))
                )
                .font(.system(size: 11)).foregroundStyle(Lab.ice)
              }
              if let column = model.statistic {
                statistic(column, report: saved.report)
              }
              interpretation(saved.report)
            }.padding(22)
          }
        }
      } else {
        Spacer()
        Image(systemName: "waveform.path").font(.system(size: 46)).foregroundStyle(Lab.lilac)
        Text(strings(model.busy ? "Reading the chain…" : "No diagnostics are available yet."))
          .font(Lab.display(22)).padding(.top, 12)
        Spacer()
      }
      if let error = model.error {
        DiagnosticMessage(title: strings(error), details: model.details, strings: strings).padding(
          18)
      }
    }
    .frame(minWidth: 960, minHeight: 700)
    .foregroundStyle(Lab.ink).background(WorkspaceBackdrop()).preferredColorScheme(.dark).tint(
      Lab.accent
    )
    .environment(
      \.visualComfort,
      VisualComfort(
        reduceMotion: preferences.reduceMotion, reduceTransparency: preferences.reduceTransparency)
    )
    .task { await model.load() }.onDisappear { model.cancel() }
    .sheet(item: $summary) { value in
      TreeSummaryView(model: value, strings: strings, onOpen: onOpenTree)
    }
  }

  private var header: some View {
    HStack(spacing: 16) {
      Image(systemName: "waveform.path.ecg").font(.system(size: 26)).foregroundStyle(Lab.ice)
        .frame(width: 54, height: 54).background(
          Lab.ice.opacity(0.1), in: RoundedRectangle(cornerRadius: 16))
      VStack(alignment: .leading, spacing: 5) {
        Text(strings("Look inside the run.")).font(Lab.display(28))
        Text(model.directory.lastPathComponent).font(.system(size: 10, design: .monospaced))
          .foregroundStyle(Lab.secondary).lineLimit(1).truncationMode(.middle)
        if let report = model.result?.report {
          Text(
            strings(
              "%@ retained · %@ discarded · first retained state %@",
              strings.number(report.retainedSamples), strings.number(report.discardedSamples),
              String(report.firstRetainedState))
          )
          .font(.system(size: 11)).foregroundStyle(Lab.ice)
        }
      }
      Spacer(minLength: 12)
      if let saved = model.result {
        Button {
          NSWorkspace.shared.activateFileViewerSelecting([
            saved.directory.appendingPathComponent("diagnostics.json")
          ])
        } label: {
          Label(strings("Calculation record"), systemImage: "doc.text")
        }
        .buttonStyle(LabButtonStyle(compact: true))
      }
    }
  }

  private var parameterList: some View {
    VStack(alignment: .leading, spacing: 14) {
      Eyebrow(text: strings("Parameters"), color: Lab.ice).padding(.horizontal, 10)
      TextField(strings("Find a parameter"), text: $model.query).textFieldStyle(.roundedBorder)
        .padding(.horizontal, 6)
      Table(
        model.columns,
        selection: Binding<String?>(
          get: { model.selected }, set: { if let name = $0 { model.selected = name } })
      ) {
        TableColumn(strings("Parameter")) { column in
          Text(column.name).font(.system(size: 11, weight: .medium)).lineLimit(1).help(column.name)
        }.width(min: 130, ideal: 155)
        TableColumn("ESS") { column in
          Text(column.ess.map(TreeFigure.number) ?? "—")
            .font(.system(size: 11, design: .monospaced))
            .accessibilityLabel(
              column.ess.map { strings("ESS %@", TreeFigure.number($0)) }
                ?? strings("ESS unavailable"))
        }.width(62)
      }.tableStyle(.inset(alternatesRowBackgrounds: false)).scrollContentBackground(.hidden)
    }.padding(.horizontal, 8).padding(.top, 16).background(Lab.surface.opacity(0.65))
  }

  private func statistic(_ column: TraceStatistic, report: TraceReport) -> some View {
    VStack(alignment: .leading, spacing: 16) {
      HStack(alignment: .firstTextBaseline) {
        Text(column.name).font(Lab.display(24)).textSelection(.enabled)
        Spacer()
        Text(strings("Within one run")).font(.system(size: 10, weight: .medium))
          .foregroundStyle(Lab.ice).padding(7).background(Lab.ice.opacity(0.08), in: Capsule())
      }
      Text(strings(explanation(column.name))).font(.system(size: 12)).foregroundStyle(Lab.secondary)
        .fixedSize(horizontal: false, vertical: true)
      HStack(spacing: 12) {
        measure("Mean", TreeFigure.number(column.mean))
        measure("Standard deviation", TreeFigure.number(column.sd))
        measure("ESS", column.ess.map(TreeFigure.number) ?? strings("Unavailable"))
      }
      HStack(spacing: 8) {
        Text(strings("95% equal-tailed interval")).foregroundStyle(Lab.secondary)
        Text(
          column.interval.map { $0.map(TreeFigure.number).joined(separator: " – ") }
            ?? strings("Unavailable")
        )
        .font(.system(size: 12, weight: .medium, design: .monospaced)).textSelection(.enabled)
      }.font(.system(size: 12))
      HStack {
        Eyebrow(text: strings("Trace"), color: Lab.ice)
        Spacer()
        Picker(strings("Plot range"), selection: $includeDiscarded) {
          Text(strings("Whole run")).tag(true)
          Text(strings("After burn-in")).tag(false)
        }.pickerStyle(.segmented).frame(width: 250).labelsHidden()
      }
      TracePlot(
        column: column, report: report, strings: strings, includeDiscarded: includeDiscarded
      )
      .id(column.name + String(includeDiscarded))
      if column.ess == nil {
        Label(
          strings(
            column.essReason == "constant"
              ? "This series is constant. Its mixing cannot be estimated from this trace."
              : "There are too few samples or insufficient numerical information to estimate ESS."),
          systemImage: "info.circle"
        )
        .font(.system(size: 12)).foregroundStyle(Lab.coral)
      }
      if column.lagLimitReached {
        Text(
          strings(
            "Positive correlation reached the estimator’s lag limit. The ESS may be optimistic; inspect a longer run."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.coral)
      }
      if report.retainedSamples < 20 {
        Text(
          strings(
            "Very few samples remain. Means and intervals are descriptive and may be unstable.")
        )
        .font(.system(size: 11)).foregroundStyle(Lab.coral)
      }
    }
  }

  private func measure(_ title: String, _ value: String) -> some View {
    VStack(alignment: .leading, spacing: 8) {
      Text(strings(title)).font(.system(size: 10)).foregroundStyle(Lab.secondary)
      Text(value).font(.system(size: 19, weight: .medium, design: .monospaced))
        .textSelection(.enabled).lineLimit(1).minimumScaleFactor(0.7)
    }.padding(14).frame(maxWidth: .infinity, alignment: .leading)
      .background(Lab.surface, in: RoundedRectangle(cornerRadius: 12))
  }

  private func interpretation(_ report: TraceReport) -> some View {
    VStack(alignment: .leading, spacing: 14) {
      Divider()
      Text(
        strings(
          "ESS describes information equivalent to independent samples, not completed MCMC steps. High scalar ESS alone does not establish agreement between runs or convergence of tree topologies."
        )
      )
      .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
      if report.source.completion == "completed" {
        Button(strings("Summarize trees with this burn-in")) {
          let value = TreeSummaryModel(directory: model.directory, executable: model.executable)
          value.burnin = String(report.burnin.value)
          summary = value
        }.buttonStyle(LabButtonStyle(compact: true)).disabled(model.busy)
      }
      DisclosureGroup(strings("Methods and source")) {
        VStack(alignment: .leading, spacing: 10) {
          Text(
            strings(
              "Statistics use every retained sample. The plotted trace is simplified for display and preserves each bucket’s extrema."
            ))
          Text(
            strings(
              "ESS uses paired positive autocovariances, with lags below 2,000. It is not rank-normalized bulk or tail ESS."
            ))
          Text(
            strings(
              "The interval uses the 2.5th and 97.5th percentiles with linear interpolation. It is not HPD and is not an interval for the Monte Carlo error of the mean."
            ))
          Text(
            strings(
              "R-hat and comparisons of clade frequencies require compatible independent runs. They are not calculated in this view."
            ))
          Text(
            strings(
              "Burn-in discards floor(sample count × percentage / 100), including STATE_0. The same selection rule is used for tree summaries."
            ))
          Text(report.source.path).textSelection(.enabled)
          Text("SHA-256: \(report.source.sha256)").font(.system(size: 10, design: .monospaced))
            .textSelection(.enabled)
        }.font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3).padding(.top, 10)
      }.font(.system(size: 11))
    }
  }

  private func explanation(_ name: String) -> String {
    switch name {
    case "posterior":
      "Log posterior density combines likelihood and priors. It is not a node’s posterior support."
    case "likelihood": "Log likelihood measures how the model and tree explain the observed matrix."
    case "prior", "birth_death", "gamma_prior", "clock_rate_prior", "clock_sigma_prior":
      "A log density contribution from the model’s prior distributions."
    case "tree_height", "tree_length":
      "Heights and lengths use the source time units; they are not automatically calendar dates."
    case "birth_difference_rate": "Net diversification rate, per source time unit."
    case "relative_death_rate": "Extinction relative to speciation; a dimensionless ratio."
    case "gamma_shape": "Dimensionless shape controlling variation in rates between characters."
    case "clock_sigma": "Standard deviation of log branch rates; a dimensionless clock parameter."
    case "mean_branch_rate": "Mean branch-rate multiplier relative to the fixed clock scale."
    default: "Values are shown in the units recorded by the source trace."
    }
  }
}

struct TracePlot: View {
  let column: TraceStatistic
  let report: TraceReport
  let strings: AppStrings
  let includeDiscarded: Bool
  let points: [TracePoint]
  let firstPlottedState: UInt64
  @State private var inspected = 0

  private let bounds: (center: Double, scale: Double, min: Double, max: Double)

  init(
    column: TraceStatistic, report: TraceReport, strings: AppStrings, includeDiscarded: Bool = true
  ) {
    self.column = column
    self.report = report
    self.strings = strings
    self.includeDiscarded = includeDiscarded
    points = column.points.filter { includeDiscarded || $0.retained }
    firstPlottedState = includeDiscarded ? report.firstState : report.firstRetainedState
    let low = points.map(\.value).min() ?? 0
    let high = points.map(\.value).max() ?? 0
    let center = low / 2 + high / 2
    bounds = (center, max(high - center, center - low), low, high)
  }
  func x(_ state: UInt64) -> Double {
    report.lastState > firstPlottedState
      ? Double(state - firstPlottedState) / Double(report.lastState - firstPlottedState) : 0.5
  }
  func y(_ value: Double) -> Double {
    bounds.scale > 0 ? (value - bounds.center) / bounds.scale : 0
  }
  private var selected: TracePoint { points[min(inspected, points.count - 1)] }

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      Chart {
        ForEach(points) { point in
          LineMark(
            x: .value(strings("Sample state"), x(point.state)),
            y: .value(column.name, y(point.value)),
            series: .value("Phase", point.retained ? "retained" : "discarded")
          )
          .foregroundStyle(point.retained ? Lab.lilac : Lab.muted.opacity(0.6))
          .lineStyle(StrokeStyle(lineWidth: 1))
        }
        if report.discardedSamples > 0 && includeDiscarded {
          RuleMark(x: .value("Burn-in", x(report.firstRetainedState)))
            .foregroundStyle(Lab.coral).lineStyle(StrokeStyle(lineWidth: 1, dash: [4, 3]))
        }
        PointMark(
          x: .value(strings("Sample state"), x(selected.state)),
          y: .value(column.name, y(selected.value))
        )
        .foregroundStyle(Lab.ice).symbolSize(35)
      }
      .chartXScale(domain: 0...1).chartYScale(domain: -1...1)
      .chartXAxis {
        AxisMarks(values: [0.0, 0.5, 1.0]) { axis in
          AxisGridLine().foregroundStyle(Lab.border.opacity(0.4))
          AxisValueLabel {
            if let value = axis.as(Double.self) {
              let state =
                value == 0
                ? firstPlottedState
                : value == 1
                  ? report.lastState
                  : firstPlottedState + (report.lastState - firstPlottedState) / 2
              Text(String(state)).font(.system(size: 10, design: .monospaced))
            }
          }
        }
      }
      .chartYAxis {
        AxisMarks(position: .leading, values: bounds.scale == 0 ? [0.0] : [-1.0, 0.0, 1.0]) {
          axis in
          AxisGridLine().foregroundStyle(Lab.border.opacity(0.4))
          AxisValueLabel {
            if let value = axis.as(Double.self) {
              Text(
                TreeFigure.number(
                  max(bounds.min, min(bounds.max, bounds.center + value * bounds.scale)))
              )
              .font(.system(size: 10, design: .monospaced))
            }
          }
        }
      }
      .chartLegend(.hidden).frame(height: 250)
      .accessibilityElement(children: .ignore)
      .accessibilityLabel(strings("Trace of %@", column.name))
      .accessibilityValue(strings("Use the sample control below to inspect plotted values."))
      HStack(spacing: 14) {
        Label(strings("Retained"), systemImage: "minus").foregroundStyle(Lab.lilac)
        Label(strings("Discarded"), systemImage: "minus").foregroundStyle(Lab.muted)
        Spacer()
        Text(strings("Sample state")).foregroundStyle(Lab.secondary)
      }.font(.system(size: 10))
      Stepper(value: $inspected, in: 0...max(0, points.count - 1)) {
        Text(
          strings(
            "State %@ · %@ · %@", String(selected.state), TreeFigure.number(selected.value),
            strings(selected.retained ? "Retained" : "Discarded"))
        )
        .font(.system(size: 11, design: .monospaced)).textSelection(.enabled)
      }.accessibilityLabel(strings("Inspect plotted samples"))
    }.padding(16).background(Lab.surface, in: RoundedRectangle(cornerRadius: 14))
  }
}
