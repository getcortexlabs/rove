import SwiftUI

struct ChainResultsView: View {
  @Environment(\.openWindow) private var openWindow
  @Bindable var workspace: Workspace
  let directory: URL
  let settings: AnalysisSettings
  let completed: Bool
  @State private var report: ChainComparison?
  @State private var error: String?
  @State private var selectedChain = 1
  private var strings: AppStrings { workspace.strings }
  private var chainDirectory: URL {
    directory.appendingPathComponent(String(format: "results/chain-%02d", selectedChain))
  }

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 20) {
        HStack {
          VStack(alignment: .leading, spacing: 7) {
            Eyebrow(text: strings("Compare the paths"), color: Lab.lilac)
            Text(strings("Do the chains agree?")).font(Lab.display(25))
          }
          Spacer()
          Image(systemName: "point.3.connected.trianglepath.dotted")
            .font(.system(size: 32, weight: .light)).foregroundStyle(Lab.lilac)
        }
        if !completed {
          Text(
            strings(
              "The chain comparison will be available when this group finishes. You can inspect each chain's partial samples below."
            )
          )
          .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
        } else if let report {
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
          Grid(alignment: .leading, horizontalSpacing: 22, verticalSpacing: 10) {
            GridRow {
              Text(strings("Statistic"))
              Text("R-hat")
              ForEach(1...settings.chains, id: \.self) { chain in
                Text(strings("Chain %@ · ESS", String(chain)))
              }
            }.font(.system(size: 10, weight: .semibold)).foregroundStyle(Lab.muted)
            Divider().gridCellColumns(settings.chains + 2)
            ForEach(report.statistics) { statistic in
              GridRow {
                Text(statistic.name).foregroundStyle(Lab.secondary)
                Text(number(statistic.rhat, digits: 3))
                  .foregroundStyle((statistic.rhat ?? .infinity) <= 1.01 ? Lab.ice : Lab.coral)
                  .help(
                    strings(
                      statistic.rhatReason == "constant"
                        ? "Constant samples; R-hat is unavailable."
                        : "Unavailable diagnostics do not count as agreement."))
                ForEach(0..<settings.chains, id: \.self) { chain in
                  Text(
                    number(statistic.ess[chain], digits: 0)
                      + (statistic.lagLimitReached[chain] ? " *" : "")
                  )
                  .foregroundStyle((statistic.ess[chain] ?? 0) >= 200 ? Lab.ice : Lab.coral)
                }
              }.font(.system(size: 11, design: .monospaced)).monospacedDigit()
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
                  ForEach(0..<settings.chains, id: \.self) { chain in
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
        } else if let error {
          DiagnosticMessage(
            title: strings("The chain comparison is unavailable."), details: error, strings: strings
          )
        } else {
          ProgressView().controlSize(.small)
        }
        Divider().overlay(Lab.border)
        HStack(spacing: 12) {
          Picker(strings("Chain outputs"), selection: $selectedChain) {
            ForEach(1...settings.chains, id: \.self) { chain in
              Text(strings("Chain %@", String(chain))).tag(chain)
            }
          }.labelsHidden().frame(width: 155)
          Text(
            strings("Seed %@", String((UInt64(settings.seed) ?? 0) &+ UInt64(selectedChain - 1)))
          )
          .font(.system(size: 10, design: .monospaced)).foregroundStyle(Lab.muted)
          Spacer()
          Button(strings("Diagnostics")) {
            workspace.openDiagnostics(in: chainDirectory)
            openWindow(id: "trace-diagnostics")
          }.buttonStyle(LabButtonStyle(compact: true))
          Button(strings("Summarize trees")) { workspace.summarizeTrees(in: chainDirectory) }
            .buttonStyle(LabButtonStyle(compact: true)).disabled(!completed)
        }
        HStack {
          Button(strings("Open chain folder")) { NSWorkspace.shared.open(chainDirectory) }
            .buttonStyle(LabButtonStyle(compact: true))
          Button(strings("Comparison file")) {
            NSWorkspace.shared.activateFileViewerSelecting([
              directory.appendingPathComponent("results/comparison.json")
            ])
          }.buttonStyle(LabButtonStyle(compact: true)).disabled(report == nil)
        }
      }
    }.task(id: [directory.path, String(completed)]) {
      report = nil
      error = nil
      guard completed else { return }
      let path = directory.appendingPathComponent("results/comparison.json")
      do {
        let value = try await Task.detached { try ChainComparison.read(path) }.value
        guard value.chainCount == settings.chains else { throw RunFailure.invalidCompletion }
        report = value
      } catch { self.error = error.localizedDescription }
    }
  }

  private func number(_ value: Double?, digits: Int) -> String {
    guard let value else { return "—" }
    return value.formatted(.number.precision(.fractionLength(digits)).locale(strings.locale))
  }
}
