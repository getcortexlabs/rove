import SwiftUI

struct RootView: View {
  @Bindable var workspace: Workspace
  private var strings: AppStrings { workspace.strings }
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
  @State private var showingHelp = false

  var body: some View {
    HStack(spacing: 0) {
      WorkspaceSidebar(workspace: workspace)
        .frame(width: 232).padding(.leading, 12).padding(.vertical, 12)
      VStack(spacing: 0) {
        toolbar
        ScrollView {
          VStack(alignment: .leading, spacing: 18) {
            if let error = workspace.error {
              HStack(alignment: .top) {
                DiagnosticMessage(
                  title: strings(error), details: workspace.errorDetails, strings: strings)
                Button {
                  workspace.error = nil
                  workspace.errorDetails = nil
                } label: {
                  Image(systemName: "xmark")
                }
                .buttonStyle(.plain).padding(.top, 14).help(strings("Dismiss error"))
              }
            }
            if workspace.showingHistory {
              HistoryView(workspace: workspace)
            } else if let run = workspace.run {
              RunView(workspace: workspace, run: run)
            } else if let report = workspace.report {
              switch workspace.page {
              case .matrix: MatrixView(workspace: workspace, report: report)
              case .model: SetupView(workspace: workspace)
              case .review: ReviewView(workspace: workspace, report: report)
              }
            } else {
              WelcomeView(workspace: workspace)
            }
          }
          .frame(maxWidth: 980).padding(.horizontal, 36).padding(.top, 12).padding(.bottom, 28)
          .frame(maxWidth: .infinity)
        }
        .id(
          workspace.showingHistory
            ? "history:\(workspace.selectedHistoryID ?? "list")"
            : "\(workspace.page.rawValue):\(workspace.report?.inputSha256 ?? "welcome")")
        if workspace.report != nil && workspace.run == nil && !workspace.showingHistory { footer }
      }
    }
    .background { WorkspaceBackdrop() }.foregroundStyle(Lab.ink)
    .preferredColorScheme(.dark).tint(Lab.accent)
    .environment(\.locale, strings.locale)
    .environment(
      \.visualComfort,
      VisualComfort(
        reduceMotion: reduceMotion || workspace.preferences.reduceMotion,
        reduceTransparency: reduceTransparency || workspace.preferences.reduceTransparency)
    )
    .frame(minWidth: 1080, minHeight: 700)
    .animation(
      reduceMotion || workspace.preferences.reduceMotion ? nil : .easeInOut(duration: 0.2),
      value: workspace.page
    )
    .task { await workspace.history.reload() }
    .overlay {
      if workspace.loading {
        ZStack {
          Lab.background.opacity(0.7).ignoresSafeArea()
          VStack(spacing: 15) {
            ProgressView().controlSize(.small).tint(Lab.accent)
            Text(strings("Getting to know your matrix…")).font(.system(size: 13, weight: .medium))
          }.padding(28).background(Lab.raised, in: RoundedRectangle(cornerRadius: 18))
            .overlay(RoundedRectangle(cornerRadius: 18).strokeBorder(Lab.border))
        }.accessibilityLabel(strings("Reading matrix"))
      }
    }
  }

  private var toolbar: some View {
    HStack(spacing: 10) {
      Image(systemName: "square.stack.3d.up").foregroundStyle(Lab.muted)
      Text(strings("Workspace")).foregroundStyle(Lab.muted)
      Text("/").foregroundStyle(Lab.border)
      Text(workspace.showingHistory ? strings("History") : workspace.title).foregroundStyle(
        Lab.secondary
      ).lineLimit(1)
      Spacer()
      if workspace.report != nil {
        Button(action: workspace.chooseFile) { Image(systemName: "folder.badge.plus") }
          .buttonStyle(.plain).foregroundStyle(Lab.secondary).help(
            strings("Choose another matrix (⌘O)")
          )
          .padding(.trailing, 12).disabled(workspace.isRunning)
      }
      HStack(spacing: 5) {
        Image(systemName: "lock.shield").font(.system(size: 10)).foregroundStyle(Lab.ice)
        Text(strings("On your Mac")).font(.system(size: 10)).foregroundStyle(Lab.secondary)
      }.padding(.trailing, 14)
      Button {
        showingHelp.toggle()
      } label: {
        Image(systemName: "questionmark.circle").font(.system(size: 15))
      }
      .buttonStyle(.plain).foregroundStyle(Lab.muted).help(strings("About this preview"))
      .popover(isPresented: $showingHelp) {
        VStack(alignment: .leading, spacing: 12) {
          Text(strings("A first look at Rove")).font(Lab.display(23))
          Text(
            strings(
              "Import a morphological NEXUS matrix, inspect its characters and prepare an analysis. The preview supports the engine's current binary Mk and Mkv models."
            )
          )
          Text(
            strings(
              "Run the Rust engine here, follow its progress and inspect the output files. You can also save and reopen a setup."
            )
          )
          Text(
            strings(
              "The example is the supplied 20 × 50 validation matrix. No clades are constrained automatically."
            )
          )
          .foregroundStyle(Lab.secondary)
        }.font(.system(size: 12)).lineSpacing(4).padding(24).frame(width: 340)
          .foregroundStyle(Lab.ink).background(Lab.surface)
      }
    }.font(.system(size: 11)).padding(.horizontal, 34).frame(height: 68)
  }

  private var footer: some View {
    VStack(spacing: 0) {
      Rectangle().fill(Lab.border.opacity(0.6)).frame(height: 1)
      HStack {
        if workspace.page != .matrix {
          Button {
            workspace.page = workspace.page == .review ? .model : .matrix
          } label: {
            Label(strings("Back"), systemImage: "arrow.left")
          }.buttonStyle(LabButtonStyle(compact: true))
        } else {
          Text(
            strings(
              "Declared states: %@ · morphological data",
              strings.number(workspace.report?.symbols.count ?? 0))
          )
          .font(.system(size: 11)).foregroundStyle(Lab.muted)
        }
        Spacer()
        Text(strings("%@ of 3", strings.number(workspace.page.number))).font(
          .system(size: 10, design: .monospaced)
        )
        .foregroundStyle(Lab.muted).padding(.trailing, 12)
        if workspace.page == .review {
          Button(action: workspace.startRun) {
            Label(strings("Start analysis"), systemImage: "play.fill")
          }
          .buttonStyle(LabButtonStyle(prominent: true)).disabled(!workspace.canStartRun)
        } else {
          Button {
            workspace.page = workspace.page == .matrix ? .model : .review
          } label: {
            HStack(spacing: 12) {
              Text(strings(workspace.page == .matrix ? "Choose a model" : "Review setup"))
              Image(systemName: "arrow.right")
            }
          }.buttonStyle(LabButtonStyle(prominent: true))
            .disabled(
              workspace.page == .matrix
                ? workspace.report?.samplerError != nil : workspace.issue != nil)
        }
      }.padding(.horizontal, 34).padding(.vertical, 18)
    }.background(Lab.background)
  }

}
