import AppKit
import SwiftUI

struct BenchmarkView: View {
  @Bindable var workspace: Workspace
  @Bindable var access: BetaAccess
  private var benchmark: Benchmark { workspace.benchmark }
  private var strings: AppStrings { workspace.strings }
  private var hardware: BenchmarkHardware { benchmark.selected?.payload.hardware ?? .current() }

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 22) {
        HStack(alignment: .top, spacing: 18) {
          Image(systemName: "speedometer").font(.system(size: 30, weight: .light))
            .foregroundStyle(Lab.accent).frame(width: 64, height: 64)
            .background(Lab.accent.opacity(0.1), in: RoundedRectangle(cornerRadius: 20))
          VStack(alignment: .leading, spacing: 7) {
            Text(strings("A little test drive.")).font(Lab.display(30, weight: .semibold))
            Text(strings("See how Rove runs on this Mac."))
              .font(.system(size: 13)).foregroundStyle(Lab.secondary)
          }
        }
        LabPanel {
          VStack(alignment: .leading, spacing: 10) {
            Text(verbatim: hardware.chip).font(Lab.display(20, weight: .semibold))
            Text(
              verbatim:
                "\(hardware.model) · \(hardware.physicalCores) CPU · \(hardware.memoryBytes / 1_073_741_824) GB · macOS \(hardware.macOS)"
            )
            .font(.system(size: 11, design: .monospaced)).foregroundStyle(Lab.secondary)
            Text(
              strings(
                "A bundled synthetic matrix: 90 taxa, 250 characters. Warm-up, then three measurements for one chain and four parallel chains. Works offline."
              )
            )
            .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
            Text(
              strings(
                "For comparable results, plug in your Mac, turn off Low Power Mode and close other demanding apps."
              )
            )
            .font(.system(size: 11)).foregroundStyle(Lab.muted).lineSpacing(3)
          }
        }
        if benchmark.isRunning {
          VStack(alignment: .leading, spacing: 12) {
            HStack {
              Text(
                strings(
                  benchmark.currentChains == 1
                    ? "Measuring one chain…" : "Measuring four parallel chains…"))
              Spacer()
              Text(verbatim: "\(min(benchmark.completedMeasurements, 6))/6")
                .monospacedDigit().foregroundStyle(Lab.accent)
            }.font(.system(size: 13, weight: .medium))
            ProgressView(value: Double(min(benchmark.completedMeasurements, 6)), total: 6).tint(
              Lab.accent)
            Text(
              strings(
                "Each measurement includes an untimed warm-up. Your analyses are left in place.")
            )
            .font(.system(size: 11)).foregroundStyle(Lab.secondary)
            Button(strings("Cancel benchmark")) { benchmark.cancel() }
              .buttonStyle(.bordered)
          }
        } else {
          HStack {
            Button(action: workspace.startBenchmark) {
              Label(strings("Run benchmark"), systemImage: "play.fill")
            }.buttonStyle(.borderedProminent)
              .disabled(!workspace.canOpenBenchmark || benchmark.sharing)
            Spacer()
            Button(strings("Saved results")) { NSWorkspace.shared.open(benchmark.store.directory) }
              .buttonStyle(.borderless).disabled(benchmark.records.isEmpty)
          }
          if !workspace.canOpenBenchmark {
            Text(strings("Finish the active analysis before starting a benchmark."))
              .font(.system(size: 12)).foregroundStyle(Lab.secondary)
          }
        }
        if !benchmark.records.isEmpty {
          Picker(
            strings("Previous tests"),
            selection: Binding(get: { benchmark.selectedID }, set: { benchmark.selectedID = $0 })
          ) {
            ForEach(benchmark.records) { record in
              Text(
                "\(Date(timeIntervalSince1970: record.payload.createdAt).formatted(date: .abbreviated, time: .shortened)) · \(strings(status(record.status)))"
              )
              .tag(Optional(record.id))
            }
          }.disabled(benchmark.isRunning || benchmark.sharing)
        }
        if let record = benchmark.selected, record.status == "completed" {
          HStack(alignment: .top, spacing: 14) {
            result(record, chains: 1)
            result(record, chains: 4)
          }
          if let engine = record.payload.engine {
            Text(
              verbatim:
                "\(record.payload.protocol?.id ?? "") · Rove \(record.payload.appVersion) (\(record.payload.appBuild)) · \(engine.backend) · \(engine.sha256.prefix(12))"
            )
            .font(.system(size: 10, design: .monospaced)).foregroundStyle(Lab.muted)
          }
          LabPanel {
            VStack(alignment: .leading, spacing: 12) {
              Text(strings("Help us understand every Mac.")).font(
                .system(size: 14, weight: .semibold))
              Text(
                strings(
                  "Sharing sends this test's timings, hardware, power and thermal conditions, and app/engine versions to Cortex Labs. No research files, serial number or beta email are included. Nothing is sent automatically."
                )
              )
              .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
              Button {
                Task { await benchmark.share(token: access.canUse ? access.session?.token : nil) }
              } label: {
                Label(
                  strings(
                    record.sharedAt != nil
                      ? "Shared. Thank you!"
                      : benchmark.sharing ? "Sharing…" : "Share with Cortex Labs"),
                  systemImage: record.sharedAt != nil ? "checkmark.circle" : "arrow.up.circle")
              }.buttonStyle(.bordered).disabled(
                record.sharedAt != nil || benchmark.sharing || benchmark.isRunning)
            }
          }
        }
        if let error = benchmark.error {
          Text(strings(error)).font(.system(size: 12)).foregroundStyle(Lab.accent).lineSpacing(4)
        }
        Text(
          strings(
            "These times measure this fixed workload, not convergence or the duration of your own analysis. Compare results only within the same protocol and engine build."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.muted).lineSpacing(4)
      }.padding(30)
    }.frame(minWidth: 700, minHeight: 680)
      .background { WorkspaceBackdrop() }.foregroundStyle(Lab.ink)
      .preferredColorScheme(.dark).tint(Lab.accent)
      .environment(\.locale, strings.locale)
      .onDisappear { benchmark.cancel() }
  }

  private func result(_ record: BenchmarkRecord, chains: Int) -> some View {
    let samples = record.payload.samples.filter { $0.chains == chains }
    let times = samples.map(\.seconds).sorted()
    let median = times.isEmpty ? 0 : times[times.count / 2]
    return LabPanel {
      VStack(alignment: .leading, spacing: 10) {
        Text(strings(chains == 1 ? "One chain" : "Four parallel chains"))
          .font(.system(size: 12, weight: .semibold)).foregroundStyle(Lab.secondary)
        Text(verbatim: median.formatted(.number.precision(.fractionLength(2))) + " s")
          .font(Lab.display(32, weight: .semibold)).foregroundStyle(Lab.accent)
        Text(strings("Median wall time")).font(.system(size: 11)).foregroundStyle(Lab.secondary)
        if median > 0 {
          Text(
            strings(
              "%@ proposals/s",
              (Double(chains * (record.payload.protocol?.measuredSteps ?? 0)) / median).formatted(
                .number.precision(.fractionLength(0))))
          )
          .font(.system(size: 12, weight: .medium))
        }
        Text(
          verbatim: samples.map {
            $0.seconds.formatted(.number.precision(.fractionLength(2))) + " s"
          }.joined(separator: " · ")
        )
        .font(.system(size: 10, design: .monospaced)).foregroundStyle(Lab.muted)
        Text(strings("All three measurements")).font(.system(size: 10)).foregroundStyle(Lab.muted)
      }.frame(maxWidth: .infinity, alignment: .leading)
    }
  }

  private func status(_ value: String) -> String {
    switch value {
    case "completed": "Completed"
    case "running": "Running"
    case "cancelled": "Cancelled"
    case "interrupted": "Interrupted"
    default: "Failed"
    }
  }
}
