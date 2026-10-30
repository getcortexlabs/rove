import SwiftUI

struct EntryView: View {
  @Bindable var workspace: Workspace
  @Bindable var access: BetaAccess
  @Bindable var experience: LaunchExperience
  @Environment(\.scenePhase) private var scenePhase
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
  private var ready: Bool { access.canUse && experience.completed }
  private var idle: Bool {
    !workspace.isRunning && !workspace.loading && workspace.treeSummary == nil
  }
  private var empty: Bool { workspace.report == nil && workspace.run == nil && !workspace.loading }

  var body: some View {
    Group {
      // Access renewal never replaces controls for a chain already running.
      if workspace.isRunning || ready {
        RootView(workspace: workspace)
      } else if access.canUse {
        OnboardingView(preferences: workspace.preferences, onFinish: finishTour)
      } else {
        BetaEntryView(access: access, preferences: workspace.preferences)
      }
    }
    .frame(minWidth: 1080, minHeight: 700)
    .background { WorkspaceBackdrop() }.foregroundStyle(Lab.ink)
    .preferredColorScheme(.dark).tint(Lab.accent)
    .environment(\.locale, workspace.strings.locale)
    .environment(
      \.visualComfort,
      VisualComfort(
        reduceMotion: reduceMotion || workspace.preferences.reduceMotion,
        reduceTransparency: reduceTransparency || workspace.preferences.reduceTransparency)
    )
    .task {
      await access.restore()
      route()
      while !Task.isCancelled {
        do { try await Task.sleep(for: .seconds(60)) } catch { return }
        await access.checkIfNeeded()
        route()
      }
    }
    .onChange(of: scenePhase) { _, phase in
      if phase == .active {
        Task {
          await access.checkIfNeeded()
          route()
        }
      }
    }
    .onChange(of: access.phase) { _, _ in route() }
    .onChange(of: workspace.isRunning) { _, _ in route() }
    .onChange(of: workspace.loading) { _, _ in route() }
    .onChange(of: workspace.treeSummary == nil) { _, _ in route() }
    .sheet(item: $experience.presentation, onDismiss: { experience.acknowledgeNews() }) {
      presentation in
      Group {
        switch presentation {
        case .tour:
          OnboardingView(
            preferences: workspace.preferences, canOpenMatrix: empty, onFinish: finishTour
          )
          .frame(width: 1080, height: 700)
        case .news:
          if let release = experience.currentRelease ?? experience.availableReleases.first {
            WhatsNewView(
              preferences: workspace.preferences, releases: experience.availableReleases,
              initial: release, canOpenExample: empty, onDone: experience.dismissNews,
              onAction: releaseAction)
          }
        }
      }
      .background { WorkspaceBackdrop() }.foregroundStyle(Lab.ink).preferredColorScheme(.dark)
      .environment(\.locale, workspace.strings.locale)
      .environment(
        \.visualComfort,
        VisualComfort(
          reduceMotion: reduceMotion || workspace.preferences.reduceMotion,
          reduceTransparency: reduceTransparency || workspace.preferences.reduceTransparency))
    }
  }

  private func route() {
    guard ready else {
      if !workspace.isRunning { experience.presentation = nil }
      return
    }
    if let url = experience.pendingURL, idle {
      experience.pendingURL = nil
      workspace.open(url)
      return
    }
    experience.presentNewsIfNeeded(allowed: access.canUse, idle: idle)
  }

  private func finishTour(_ action: LaunchExperience.TourAction) {
    experience.completeTour()
    guard access.canUse, !workspace.isRunning else { return }
    if let url = experience.pendingURL {
      experience.pendingURL = nil
      workspace.open(url)
      return
    }
    // Defer panel presentation until a manually reopened tour has dismissed.
    Task { @MainActor in
      await Task.yield()
      switch action {
      case .workspace: break
      case .example: if empty { workspace.loadExample() }
      case .importMatrix: if empty { workspace.chooseFile() }
      }
    }
  }

  private func releaseAction(_ destination: ReleaseHighlight.Destination) {
    experience.dismissNews()
    guard access.canUse, idle else { return }
    switch destination {
    case .history: workspace.openHistory()
    case .guide: NSWorkspace.shared.open(workspace.strings.guideURL)
    case .example:
      // Viewing release notes must not replace an open matrix or its settings.
      if empty { workspace.loadExample() }
    }
  }
}
