import SwiftUI

struct HistoryView: View {
  @Bindable var workspace: Workspace
  @State private var search = ""
  @State private var filter = "all"
  private var strings: AppStrings { workspace.strings }
  private var history: RunHistory { workspace.history }
  private var entries: [HistoryEntry] {
    history.entries.filter {
      (!workspace.isRunning
        || $0.directory
          != workspace.run?.request.directory.standardizedFileURL.resolvingSymlinksInPath())
        && (filter == "all" || $0.status.rawValue == filter)
        && (search.isEmpty || $0.name.localizedCaseInsensitiveContains(search)
          || $0.draft?.settings.seed.localizedCaseInsensitiveContains(search) == true)
    }
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 24) {
      if let selected = history.entries.first(where: { $0.id == workspace.selectedHistoryID }) {
        Button {
          workspace.selectedHistoryID = nil
        } label: {
          Label(strings("All runs"), systemImage: "arrow.left")
        }.buttonStyle(LabButtonStyle(compact: true))
        HistoryDetail(workspace: workspace, entry: selected)
      } else {
        header
        if let run = workspace.run, run.isActive {
          Button(action: workspace.returnToWorkspace) {
            HStack(spacing: 14) {
              Image(systemName: "waveform.path").font(.system(size: 23)).foregroundStyle(Lab.accent)
              VStack(alignment: .leading, spacing: 5) {
                Text(strings("Your analysis is still running.")).font(
                  .system(size: 13, weight: .semibold))
                Text(URL(fileURLWithPath: run.request.draft.matrixPath).lastPathComponent).font(
                  .system(size: 11)
                ).foregroundStyle(Lab.secondary)
              }
              Spacer()
              Text(strings("View progress")).font(.system(size: 12))
              Image(systemName: "arrow.right")
            }.padding(18).background(
              Lab.accent.opacity(0.07), in: RoundedRectangle(cornerRadius: 14))
          }.buttonStyle(.plain)
        }
        if !history.unavailableRoots.isEmpty {
          DiagnosticMessage(
            title: strings("Some run folders could not be read."),
            details: history.unavailableRoots.joined(separator: "\n"), strings: strings)
        }
        if history.entries.isEmpty && !workspace.isRunning {
          emptyState
        } else {
          HStack(spacing: 12) {
            TextField(strings("Find a matrix or seed"), text: $search)
              .textFieldStyle(.roundedBorder).frame(maxWidth: 310)
            Spacer()
            Picker(strings("Run status"), selection: $filter) {
              Text(strings("All runs")).tag("all")
              ForEach(HistoryStatus.allCases, id: \.self) { status in
                Text(strings(status.rawValue)).tag(status.rawValue)
              }
            }.labelsHidden().frame(width: 180)
          }
          if entries.isEmpty {
            Text(strings("No runs match this search.")).font(.system(size: 13)).foregroundStyle(
              Lab.secondary
            ).padding(.vertical, 30)
          }
          LazyVStack(spacing: 12) {
            ForEach(entries) { entry in
              Button {
                workspace.selectedHistoryID = entry.id
              } label: {
                row(entry)
              }
              .buttonStyle(.plain)
              .accessibilityLabel(
                "\(entry.name), \(strings(entry.status.rawValue)), \(date(entry.startedAt))"
              )
              .help(strings("View run details"))
            }
          }
        }
      }
    }.padding(.top, 16)
  }

  private var header: some View {
    VStack(alignment: .leading, spacing: 18) {
      HStack(alignment: .top) {
        VStack(alignment: .leading, spacing: 10) {
          Eyebrow(text: strings("Your field notes"), color: Lab.accent)
          Text(strings("Every exploration has a story.")).font(Lab.display(32)).tracking(-1)
          Text(strings("Revisit your runs, keep their context, and pick a new starting point."))
            .font(.system(size: 12)).foregroundStyle(Lab.secondary)
        }
        Spacer(minLength: 12)
        if let image = Lab.explorerImage {
          Image(nsImage: image).resizable().scaledToFit().frame(width: 94, height: 100)
            .accessibilityHidden(true)
        }
      }
      HStack(spacing: 10) {
        Button(action: workspace.addHistoryFolder) {
          Label(strings("Add run folder"), systemImage: "folder.badge.plus")
        }.buttonStyle(LabButtonStyle(compact: true))
        Button {
          Task { await history.reload() }
        } label: {
          Label(strings("Refresh"), systemImage: "arrow.clockwise")
        }.buttonStyle(LabButtonStyle(compact: true)).disabled(history.loading)
        if history.loading {
          ProgressView().controlSize(.small).accessibilityLabel(strings("Reading history"))
        }
        Spacer()
        Button(action: workspace.returnToWorkspace) {
          Text(strings("Back to workspace"))
        }.buttonStyle(LabButtonStyle(compact: true))
      }
    }
  }

  private var emptyState: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 16) {
        Image(systemName: "clock.badge.checkmark").font(.system(size: 36, weight: .light))
          .foregroundStyle(Lab.ice)
        Text(strings(history.loading ? "Reading history" : "Your next run starts the collection."))
          .font(Lab.display(24))
        Text(
          strings(
            "Runs saved by Rove appear here, even after you close the app. You can also add a run folder you already have."
          )
        )
        .font(.system(size: 13)).foregroundStyle(Lab.secondary).lineSpacing(4)
        Button(action: workspace.returnToWorkspace) {
          Label(strings("Back to workspace"), systemImage: "arrow.right")
        }.buttonStyle(LabButtonStyle(prominent: true))
      }.padding(.vertical, 18).frame(maxWidth: .infinity, alignment: .leading)
    }
  }

  private func row(_ entry: HistoryEntry) -> some View {
    HStack(spacing: 16) {
      Image(systemName: "doc.text.image").font(.system(size: 23, weight: .light)).foregroundStyle(
        Lab.lilac
      )
      .frame(width: 44, height: 50).background(
        Lab.accent.opacity(0.07), in: RoundedRectangle(cornerRadius: 12))
      VStack(alignment: .leading, spacing: 8) {
        Text(entry.name).font(.system(size: 14, weight: .semibold)).lineLimit(1)
        Text(date(entry.startedAt)).font(.system(size: 11)).foregroundStyle(Lab.secondary)
        if let draft = entry.draft {
          Text(
            "\(strings(draft.settings.clock == "reference" ? "Relaxed clock + gamma" : "Strict clock")) · \(strings("Seed %@", draft.settings.seed))"
          )
          .font(.system(size: 10)).foregroundStyle(Lab.muted)
        }
      }
      Spacer(minLength: 12)
      VStack(alignment: .trailing, spacing: 10) {
        HistoryStatusBadge(status: entry.status, strings: strings)
        Text(HistoryDetail.duration(entry.elapsedSeconds)).font(
          .system(size: 11, design: .monospaced)
        ).foregroundStyle(Lab.secondary)
      }
      Image(systemName: "chevron.right").font(.system(size: 11)).foregroundStyle(Lab.muted)
    }.padding(18).background(Lab.surface, in: RoundedRectangle(cornerRadius: 15))
      .overlay(RoundedRectangle(cornerRadius: 15).strokeBorder(Lab.border.opacity(0.7)))
      .contentShape(RoundedRectangle(cornerRadius: 15))
  }

  private func date(_ value: Date?) -> String {
    value?.formatted(
      .dateTime.day().month(.abbreviated).year().hour().minute().locale(strings.locale))
      ?? strings("Date unavailable")
  }
}

private struct HistoryStatusBadge: View {
  let status: HistoryStatus
  let strings: AppStrings
  var body: some View {
    Text(strings(status.rawValue)).font(.system(size: 10, weight: .medium))
      .foregroundStyle(status == .completed ? Lab.ice : (status == .failed ? Lab.coral : Lab.lilac))
      .padding(.horizontal, 9).padding(.vertical, 6)
      .background(.white.opacity(0.045), in: Capsule())
  }
}

private struct HistoryDetail: View {
  @Environment(\.openWindow) private var openWindow
  @Bindable var workspace: Workspace
  let entry: HistoryEntry
  private var strings: AppStrings { workspace.strings }

  var body: some View {
    VStack(alignment: .leading, spacing: 22) {
      VStack(alignment: .leading, spacing: 10) {
        HStack {
          Eyebrow(text: strings("Run details"), color: Lab.accent)
          Spacer()
          HistoryStatusBadge(status: entry.status, strings: strings)
        }
        Text(entry.name).font(Lab.display(30)).textSelection(.enabled)
        if let date = entry.startedAt {
          Text(
            date.formatted(
              .dateTime.day().month(.abbreviated).year().hour().minute().locale(strings.locale))
          )
          .font(.system(size: 12)).foregroundStyle(Lab.secondary)
        }
      }
      if let message = entry.message {
        InlineMessage(text: strings(message), isError: entry.status != .interrupted)
      }
      if let checkpoint = entry.checkpoint {
        LabPanel {
          HStack(spacing: 18) {
            Image(systemName: "bookmark.circle.fill").font(.system(size: 28)).foregroundStyle(
              Lab.lilac)
            VStack(alignment: .leading, spacing: 6) {
              Text(strings("Your place is saved")).font(.system(size: 14, weight: .medium))
              Text(
                strings(
                  "Resume from step %@ with the original settings.",
                  strings.number(Int(checkpoint.step)))
              )
              .font(.system(size: 11)).foregroundStyle(Lab.secondary)
            }
            Spacer()
            Button {
              workspace.resumeRun(entry)
            } label: {
              Label(strings("Resume analysis"), systemImage: "play.fill")
            }.buttonStyle(LabButtonStyle(prominent: true)).disabled(
              workspace.isRunning || workspace.loading)
          }
        }
      }
      LabPanel {
        VStack(alignment: .leading, spacing: 16) {
          HStack(spacing: 36) {
            metric("Elapsed", Self.duration(entry.elapsedSeconds))
            if let draft = entry.draft {
              metric(
                "Completed steps",
                entry.completedSteps.map {
                  strings("%@ of %@", strings.number(Int($0)), strings.number(draft.settings.steps))
                } ?? "—")
              metric("Random seed", draft.settings.seed)
            }
          }
          if let draft = entry.draft {
            Divider().overlay(Lab.border)
            field(
              "Evolutionary clock",
              strings(
                draft.settings.clock == "reference"
                  ? "Lognormal relaxed clock · 4 gamma categories"
                  : "Strict clock · 1 rate category"))
            field(
              "Character model",
              strings(
                draft.settings.conditioning == "variable"
                  ? "Mkv · variable characters" : "Mk · all characters"))
            field(
              "State partitions",
              strings(
                draft.settings.partitioning == "observed"
                  ? "By observed states" : "Declared alphabet"))
            field("Independent chains", String(draft.settings.chains))
            field("Sampling", strings("Every %@ steps", strings.number(draft.settings.sampleEvery)))
            field("Gaps", strings(draft.settings.gapsAsMissing ? "Treat as missing" : "Reject"))
            Divider().overlay(Lab.border)
            if draft.settings.dating.enabled {
              DatingSummary(dating: draft.settings.dating, strings: strings)
              Divider().overlay(Lab.border)
            }
            ConstraintSummary(
              outgroup: draft.outgroup, taxa: draft.taxa,
              groups: draft.constraints, strings: strings)
            DisclosureGroup(strings("Input details")) {
              VStack(alignment: .leading, spacing: 8) {
                Text(draft.matrixPath)
                Text("SHA-256: \(draft.matrixSHA256)")
              }.font(.system(size: 10, design: .monospaced)).foregroundStyle(Lab.secondary)
                .textSelection(.enabled).padding(.top, 8).frame(
                  maxWidth: .infinity, alignment: .leading)
            }.font(.system(size: 11))
          }
        }
      }
      Text(
        strings(
          "A completed run records finished steps, not evidence of convergence. Review sampling diagnostics before interpreting the result."
        )
      )
      .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
      LabPanel {
        VStack(alignment: .leading, spacing: 14) {
          HStack {
            Eyebrow(text: strings("Run folder"))
            Spacer()
            Button {
              NSWorkspace.shared.open(entry.directory)
            } label: {
              Label(strings("Open folder"), systemImage: "folder")
            }.buttonStyle(LabButtonStyle(compact: true)).disabled(!entry.folderAvailable)
          }
          Text(entry.directory.path).font(.system(size: 10, design: .monospaced)).foregroundStyle(
            Lab.secondary
          )
          .textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
          ViewThatFits(in: .horizontal) {
            HStack(spacing: 8) { outputButtons }
            VStack(alignment: .leading, spacing: 8) { outputButtons }
          }
        }
      }
      if entry.files.contains("results/trace.log") {
        Button(strings("Diagnostics")) {
          workspace.openDiagnostics(in: entry.directory)
          openWindow(id: "trace-diagnostics")
        }.buttonStyle(LabButtonStyle(compact: true))
      }
      if let draft = entry.draft, draft.settings.chains > 1 {
        ChainResultsView(
          workspace: workspace, directory: entry.directory,
          settings: draft.settings, completed: entry.status == .completed)
      }
      if entry.status == .completed && entry.draft?.settings.chains == 1 {
        Button(strings("Summarize trees")) { workspace.summarizeTrees(in: entry.directory) }
          .buttonStyle(LabButtonStyle(prominent: true))
      }
      if entry.status == .completed, entry.draft != nil {
        RunComparisonView(workspace: workspace, entry: entry)
      }
      HStack(alignment: .center, spacing: 18) {
        Button {
          workspace.reuseSetup(entry)
        } label: {
          Label(strings("Use this setup"), systemImage: "arrow.up.doc")
        }.buttonStyle(LabButtonStyle(prominent: true))
          .disabled(entry.draft == nil || workspace.isRunning || workspace.loading)
        Text(
          strings(
            workspace.isRunning
              ? "Finish or cancel the current analysis before reusing a setup."
              : "Opens the configuration for review and checks the original matrix. Your next run starts from the beginning."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
      }
    }
  }

  @ViewBuilder private var outputButtons: some View {
    if entry.draft?.settings.chains == 1 {
      output("Trace", "results/trace.log")
      output("Trees", "results/trees.nexus")
    }
    output("Run record", "results/run.json")
    output("Session record", "session.json")
    output("Engine log", "engine.stderr.log")
  }

  private func output(_ label: String, _ path: String) -> some View {
    Button {
      NSWorkspace.shared.activateFileViewerSelecting([entry.directory.appendingPathComponent(path)])
    } label: {
      Label(strings(label), systemImage: "doc.text")
    }.buttonStyle(LabButtonStyle(compact: true)).disabled(!entry.files.contains(path))
  }

  private func metric(_ label: String, _ value: String) -> some View {
    VStack(alignment: .leading, spacing: 8) {
      Eyebrow(text: strings(label))
      Text(value).font(.system(size: 16, weight: .medium, design: .monospaced))
    }
  }

  private func field(_ label: String, _ value: String) -> some View {
    HStack(alignment: .top) {
      Text(strings(label)).foregroundStyle(Lab.secondary)
      Spacer(minLength: 20)
      Text(value).multilineTextAlignment(.trailing).textSelection(.enabled)
    }.font(.system(size: 12))
  }

  static func duration(_ value: Double?) -> String {
    guard let value else { return "—" }
    let seconds = Int(min(value, Double(Int.max / 2)))
    return String(format: "%02d:%02d:%02d", seconds / 3600, (seconds / 60) % 60, seconds % 60)
  }
}
