import SwiftUI

/// The statistics grid and clade agreement of a comparison between chains or runs.
struct ComparisonReportView: View {
  let report: ChainComparison
  let labels: [String]
  let strings: AppStrings

  var body: some View {
    VStack(alignment: .leading, spacing: 20) {
      Text(
        strings(
          "%@ samples per chain after %@%% burn-in.", strings.number(report.retainedPerChain),
          String(report.burninPercent))
      )
      .font(.system(size: 12)).foregroundStyle(Lab.secondary)
      Text(
        strings(
          "R-hat near 1 means the chains look similar for that statistic. ESS estimates useful samples within each chain. Neither proves that all possible trees were explored."
        )
      )
      .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(4)
      ScrollView(.horizontal) {
        Grid(alignment: .leading, horizontalSpacing: 22, verticalSpacing: 10) {
          GridRow {
            Text(strings("Statistic"))
            Text("R-hat")
            ForEach(labels.indices, id: \.self) { index in
              Text(strings("%@ · ESS", labels[index]))
            }
          }.font(.system(size: 10, weight: .semibold)).foregroundStyle(Lab.muted)
          Divider().gridCellColumns(report.chainCount + 2)
          ForEach(report.statistics) { statistic in
            GridRow {
              Text(statistic.name).foregroundStyle(Lab.secondary)
              Text(
                statistic.fixedByModel == true
                  ? strings("Fixed") : number(statistic.rhat, digits: 3)
              )
              .foregroundStyle(
                statistic.fixedByModel == true
                  ? Lab.secondary : ((statistic.rhat ?? .infinity) <= 1.01 ? Lab.ice : Lab.coral)
              )
              .help(
                strings(
                  statistic.fixedByModel == true
                    ? "Fixed by the model; sampling diagnostics do not apply."
                    : statistic.rhatReason == "constant"
                      ? "Constant samples; R-hat is unavailable."
                      : "Unavailable diagnostics do not count as agreement."))
              ForEach(0..<report.chainCount, id: \.self) { chain in
                Text(
                  number(statistic.ess[chain], digits: 0)
                    + (statistic.lagLimitReached[chain] ? " *" : "")
                )
                .foregroundStyle(
                  statistic.fixedByModel == true
                    ? Lab.secondary : ((statistic.ess[chain] ?? 0) >= 200 ? Lab.ice : Lab.coral))
              }
            }.font(.system(size: 11, design: .monospaced)).monospacedDigit()
          }
        }
      }.frame(maxWidth: .infinity, alignment: .leading)
      Text(
        strings(
          "— means unavailable. * means the ESS autocorrelation estimate reached its lag limit. Display guides: ESS 200 and R-hat 1.01."
        )
      )
      .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
      DisclosureGroup(strings("Clade agreement")) {
        VStack(alignment: .leading, spacing: 12) {
          Text(
            strings(
              "Largest difference between unconstrained clade frequencies: %@ percentage points.",
              number(report.maxUnconstrainedCladeDifference * 100, digits: 2))
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary)
          Text(
            strings(
              "Groups imposed in the setup are marked as constrained. Frequencies that stay at 0% or 100% do not establish mixing between trees."
            )
          )
          .font(.system(size: 10)).foregroundStyle(Lab.muted).lineSpacing(3)
          ForEach(Array(report.clades.prefix(20))) { clade in
            HStack(alignment: .top) {
              VStack(alignment: .leading, spacing: 3) {
                Text(clade.taxa.joined(separator: ", ")).lineLimit(2)
                if clade.constrained {
                  Text(strings("Constrained")).foregroundStyle(Lab.lilac)
                } else if clade.constantInEveryChain {
                  Text(strings("Constant within each chain")).foregroundStyle(Lab.muted)
                }
              }.frame(maxWidth: .infinity, alignment: .leading)
              ForEach(0..<report.chainCount, id: \.self) { chain in
                Text(number(clade.frequencies[chain] * 100, digits: 1) + "%")
                  .frame(width: 64, alignment: .trailing)
              }
            }.font(.system(size: 10, design: .monospaced))
          }
          Text(
            strings(
              "Showing up to 20 clades, largest differences first. The comparison file contains every observed clade."
            )
          )
          .font(.system(size: 10)).foregroundStyle(Lab.muted)
        }.padding(.top, 12)
      }.font(.system(size: 12))
    }
  }

  private func number(_ value: Double?, digits: Int) -> String {
    guard let value else { return "—" }
    return value.formatted(.number.precision(.fractionLength(digits)).locale(strings.locale))
  }
}
