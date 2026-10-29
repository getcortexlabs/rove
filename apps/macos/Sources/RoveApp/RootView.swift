import SwiftUI

struct RootView: View {
  @Bindable var workspace: Workspace
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @State private var showingHelp = false

  var body: some View {
    HStack(spacing: 0) {
      sidebar.frame(width: 218)
      Rectangle().fill(Lab.border.opacity(0.65)).frame(width: 1)
      VStack(spacing: 0) {
        toolbar
        Rectangle().fill(Lab.border.opacity(0.5)).frame(height: 1)
        ScrollView {
          VStack(alignment: .leading, spacing: 18) {
            if let error = workspace.error {
              HStack(alignment: .top) {
                InlineMessage(text: error, isError: true)
                Button {
                  workspace.error = nil
                } label: {
                  Image(systemName: "xmark")
                }
                .buttonStyle(.plain).padding(.top, 14).help("Dismiss error")
              }
            }
            if let report = workspace.report {
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
        .id("\(workspace.page.rawValue):\(workspace.report?.inputSha256 ?? "welcome")")
        if workspace.report != nil { footer }
      }
    }
    .background(Lab.background).foregroundStyle(Lab.ink)
    .preferredColorScheme(.dark).tint(Lab.lime)
    .frame(minWidth: 1080, minHeight: 700)
    .animation(reduceMotion ? nil : .easeInOut(duration: 0.2), value: workspace.page)
    .onChange(of: workspace.settings) {
      workspace.savedURL = nil
      workspace.notice = nil
    }
    .overlay {
      if workspace.loading {
        ZStack {
          Lab.background.opacity(0.7).ignoresSafeArea()
          VStack(spacing: 15) {
            ProgressView().controlSize(.small).tint(Lab.lime)
            Text("Getting to know your matrix…").font(.system(size: 13, weight: .medium))
          }.padding(28).background(Lab.raised, in: RoundedRectangle(cornerRadius: 18))
            .overlay(RoundedRectangle(cornerRadius: 18).strokeBorder(Lab.border))
        }.accessibilityLabel("Reading matrix")
      }
    }
  }

  private var sidebar: some View {
    VStack(alignment: .leading, spacing: 0) {
      HStack(spacing: 9) {
        if let url = Lab.resource("RoveLogo", extension: "png"), let logo = NSImage(contentsOf: url)
        {
          Image(nsImage: logo).resizable().scaledToFit().frame(width: 34, height: 37)
            .accessibilityHidden(true)
        }
        Text("rove").font(Lab.display(31, weight: .semibold)).tracking(-1.3)
        Spacer()
        Text("LAB").font(.system(size: 8, weight: .medium, design: .monospaced)).tracking(1)
          .foregroundStyle(Lab.lime).padding(.horizontal, 6).padding(.vertical, 4)
          .overlay(Capsule().strokeBorder(Lab.lime.opacity(0.3)))
      }.padding(.top, 54).padding(.horizontal, 22).padding(.bottom, 31)

      Button(action: workspace.reset) {
        HStack {
          Image(systemName: "plus").font(.system(size: 12, weight: .semibold))
          Text("New analysis").font(.system(size: 12, weight: .medium))
          Spacer()
          Text("⌘N").font(.system(size: 10)).foregroundStyle(Lab.muted)
        }.foregroundStyle(Lab.ink).padding(12).background(
          Lab.raised, in: RoundedRectangle(cornerRadius: 10)
        )
        .overlay(RoundedRectangle(cornerRadius: 10).strokeBorder(Lab.border.opacity(0.7)))
      }.buttonStyle(.plain).padding(.horizontal, 15)

      Eyebrow(text: "Your exploration").padding(.horizontal, 24).padding(.top, 30).padding(
        .bottom, 13)
      ForEach(WorkspacePage.allCases, id: \.self) { page in
        Button {
          if workspace.canOpen(page) { workspace.page = page }
        } label: {
          HStack(spacing: 11) {
            Image(systemName: icon(page)).font(.system(size: 13)).frame(width: 18)
            Text(page.rawValue).font(
              .system(size: 12, weight: workspace.page == page ? .semibold : .regular))
            Spacer()
            if workspace.report != nil && workspace.page.number > page.number {
              Image(systemName: "checkmark").font(.system(size: 10, weight: .medium))
            } else {
              Text(String(format: "%02d", page.number)).font(.system(size: 9, design: .monospaced))
            }
          }.foregroundStyle(workspace.page == page ? Lab.lime : Lab.secondary)
            .padding(.horizontal, 12).padding(.vertical, 12)
            .background(
              workspace.page == page ? Lab.lime.opacity(0.08) : .clear,
              in: RoundedRectangle(cornerRadius: 9)
            )
            .contentShape(Rectangle())
        }.buttonStyle(.plain).padding(.horizontal, 15).padding(.bottom, 3)
          .disabled(!workspace.canOpen(page))
          .accessibilityAddTraits(workspace.page == page ? .isSelected : [])
      }

      Eyebrow(text: "From the lab").padding(.horizontal, 24).padding(.top, 29).padding(.bottom, 14)
      Button(action: workspace.loadExample) {
        HStack(spacing: 11) {
          Image(systemName: "sparkles").foregroundStyle(Lab.lilac).frame(width: 18)
          VStack(alignment: .leading, spacing: 4) {
            Text("Example matrix").font(.system(size: 12)).foregroundStyle(Lab.ink)
            Text("20 terminals · 50 characters").font(.system(size: 9)).foregroundStyle(Lab.muted)
          }
        }.frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 26).padding(
          .vertical, 9
        )
        .contentShape(Rectangle())
      }.buttonStyle(.plain)
      Spacer(minLength: 25)

      Button {
        NSWorkspace.shared.open(URL(string: "https://rove.getcortexlabs.com/guide/")!)
      } label: {
        VStack(alignment: .leading, spacing: 10) {
          HStack {
            Image(systemName: "book.closed").foregroundStyle(Lab.lilac)
            Spacer()
            Image(systemName: "arrow.up.right").foregroundStyle(Lab.muted)
          }
          Text("A little field guide").font(Lab.display(17, weight: .semibold)).foregroundStyle(
            Lab.ink)
          Text("Big ideas, explained one\nsmall step at a time.").font(.system(size: 11))
            .foregroundStyle(Lab.secondary).lineSpacing(3)
        }.padding(16).frame(maxWidth: .infinity, alignment: .leading)
          .background(Lab.lilac.opacity(0.045), in: RoundedRectangle(cornerRadius: 12))
          .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Lab.lilac.opacity(0.12)))
      }.buttonStyle(.plain).padding(15)
      Button {
        NSWorkspace.shared.open(URL(string: "https://getcortexlabs.com/")!)
      } label: {
        HStack(spacing: 7) {
          Text("✳").font(.system(size: 21)).foregroundStyle(Lab.lime)
          Text("CORTEX LABS").font(.system(size: 9, weight: .medium, design: .monospaced)).tracking(
            1.4)
          Spacer()
          Text("↗").font(.system(size: 11))
        }.foregroundStyle(Lab.muted).padding(.horizontal, 24).padding(.top, 9).padding(.bottom, 24)
      }.buttonStyle(.plain)
    }.background(Lab.sidebar)
  }

  private var toolbar: some View {
    HStack(spacing: 10) {
      Text("Workspace").foregroundStyle(Lab.muted)
      Text("/").foregroundStyle(Lab.border)
      Text(workspace.title).foregroundStyle(Lab.secondary).lineLimit(1)
      Spacer()
      if workspace.report != nil {
        Button(action: workspace.chooseFile) { Image(systemName: "folder.badge.plus") }
          .buttonStyle(.plain).foregroundStyle(Lab.secondary).help("Choose another matrix (⌘O)")
          .padding(.trailing, 12)
      }
      HStack(spacing: 5) {
        Circle().fill(Lab.lime).frame(width: 5, height: 5)
        Text("On your Mac").font(.system(size: 10)).foregroundStyle(Lab.secondary)
      }.padding(.trailing, 14)
      Button {
        showingHelp.toggle()
      } label: {
        Image(systemName: "questionmark.circle").font(.system(size: 15))
      }
      .buttonStyle(.plain).foregroundStyle(Lab.muted).help("About this preview")
      .popover(isPresented: $showingHelp) {
        VStack(alignment: .leading, spacing: 12) {
          Text("A first look at Rove").font(Lab.display(23))
          Text(
            "Import a morphological NEXUS matrix, inspect its characters and prepare an analysis. The preview supports the engine's current binary Mk and Mkv models."
          )
          Text(
            "Save your setup or copy a Terminal command. Running and monitoring analyses inside the app will come in a later version."
          )
          Text(
            "The example is the supplied 20 × 50 validation matrix. No clades are constrained automatically."
          )
          .foregroundStyle(Lab.secondary)
        }.font(.system(size: 12)).lineSpacing(4).padding(24).frame(width: 340)
          .foregroundStyle(Lab.ink).background(Lab.surface)
      }
    }.font(.system(size: 11)).padding(.horizontal, 34).frame(height: 62)
  }

  private var footer: some View {
    VStack(spacing: 0) {
      Rectangle().fill(Lab.border.opacity(0.6)).frame(height: 1)
      HStack {
        if workspace.page != .matrix {
          Button {
            workspace.page = workspace.page == .review ? .model : .matrix
          } label: {
            Label("Back", systemImage: "arrow.left")
          }.buttonStyle(LabButtonStyle(compact: true))
        } else {
          Text("\(workspace.report?.symbols.count ?? 0) declared states · morphological data")
            .font(.system(size: 11)).foregroundStyle(Lab.muted)
        }
        Spacer()
        Text("\(workspace.page.number) of 3").font(.system(size: 10, design: .monospaced))
          .foregroundStyle(Lab.muted).padding(.trailing, 12)
        if workspace.page == .review {
          Button(action: workspace.saveDraft) {
            Label("Save setup", systemImage: "square.and.arrow.down")
          }
          .buttonStyle(LabButtonStyle(prominent: true)).disabled(workspace.issue != nil)
        } else {
          Button {
            workspace.page = workspace.page == .matrix ? .model : .review
          } label: {
            HStack(spacing: 12) {
              Text(workspace.page == .matrix ? "Choose a model" : "Review setup")
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

  private func icon(_ page: WorkspacePage) -> String {
    switch page {
    case .matrix: "square.grid.3x3"
    case .model: "slider.horizontal.3"
    case .review: "checkmark.seal"
    }
  }
}
