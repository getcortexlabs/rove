import SwiftUI

struct OnboardingView: View {
  @Bindable var preferences: AppPreferences
  var canOpenMatrix = true
  var onFinish: (LaunchExperience.TourAction) -> Void
  @State var step = 0
  @Environment(\.visualComfort) private var comfort
  private var strings: AppStrings { preferences.strings }
  private let chapters = [
    "Meet Rove", "Bring the clues", "Ask your question", "Follow the evidence",
  ]
  private let titles = [
    "Small clues.\nBig discoveries.", "Every character\nhas a story.", "Choose where\nto begin.",
    "Your research.\nRoom to explore.",
  ]
  private let descriptions = [
    "Rove explores evolutionary relationships from morphological data. A native workspace, a Rust engine, and a curious little companion.",
    "Rows are taxa. Columns are characters. Bring a NEXUS matrix and Rove will help you inspect the states and missing observations before you run anything.",
    "Review the model and choose your outgroup explicitly. It gives your analysis a rooting constraint; Rove never guesses one for you.",
    "Follow the run, inspect its traces and explore the trees. History keeps your work together, with checkpoints for picking up a paused analysis.",
  ]

  var body: some View {
    VStack(spacing: 0) {
      ArrivalHeader(preferences: preferences, caption: "YOUR FIRST FIELD TRIP")
      HStack(alignment: .center, spacing: 42) {
        VStack(alignment: .leading, spacing: 15) {
          Eyebrow(text: strings("A SMALL INTRODUCTION"), color: Lab.ice)
          Text(strings("Let's find\nyour bearings."))
            .font(Lab.display(30)).tracking(-0.9).lineSpacing(1).fixedSize(
              horizontal: false, vertical: true
            ).padding(.bottom, 10)
          ForEach(chapters.indices, id: \.self) { index in
            Button {
              go(to: index)
            } label: {
              HStack(spacing: 13) {
                ZStack {
                  Circle().fill(index == step ? Lab.accent : Lab.raised)
                  if index < step {
                    Image(systemName: "checkmark").font(.system(size: 10, weight: .semibold))
                  } else {
                    Text(String(format: "%02d", index + 1)).font(
                      .system(size: 10, weight: .medium, design: .monospaced))
                  }
                }.frame(width: 31, height: 31).foregroundStyle(
                  index == step ? Lab.background : Lab.secondary)
                Text(strings(chapters[index])).font(
                  .system(size: 12, weight: index == step ? .semibold : .regular))
                Spacer(minLength: 2)
              }.padding(11).foregroundStyle(index == step ? Lab.ink : Lab.muted)
                .background(
                  index == step ? Lab.accent.opacity(0.07) : .clear,
                  in: RoundedRectangle(cornerRadius: 13)
                )
                .contentShape(Rectangle())
            }.buttonStyle(.plain).accessibilityAddTraits(index == step ? .isSelected : [])
          }
          Spacer(minLength: 26)
          Text(strings("Built from conversations\nbetween a developer and a biologist."))
            .font(.system(size: 11)).foregroundStyle(Lab.muted).lineSpacing(4).fixedSize(
              horizontal: false, vertical: true)
          Text(verbatim: "Rafael Augusto · Thiago J. Silva")
            .font(.system(size: 10, weight: .medium)).foregroundStyle(Lab.secondary)
        }.frame(width: 235, height: 470, alignment: .topLeading)

        VStack(alignment: .leading, spacing: 0) {
          ScrollView {
            HStack(alignment: .top, spacing: 28) {
              VStack(alignment: .leading, spacing: 17) {
                Text(strings(chapters[step]).uppercased())
                  .font(.system(size: 9, weight: .medium, design: .monospaced)).tracking(1.8)
                  .foregroundStyle(Lab.ice)
                Text(strings(titles[step])).font(Lab.display(38)).tracking(-1.2)
                  .fixedSize(horizontal: false, vertical: true)
                Text(strings(descriptions[step])).font(.system(size: 13)).lineSpacing(5)
                  .foregroundStyle(Lab.secondary).fixedSize(horizontal: false, vertical: true)
              }.frame(maxWidth: .infinity, alignment: .leading)
              if step == 0 {
                ExplorerArtwork(strings: strings).scaleEffect(0.63).frame(width: 185, height: 230)
              }
            }.padding(30)
            illustration.padding(.horizontal, 30).padding(.bottom, 28)
          }
          Rectangle().fill(Lab.border.opacity(0.5)).frame(height: 1)
          HStack(spacing: 12) {
            if step > 0 {
              Button {
                go(to: step - 1)
              } label: {
                Image(systemName: "arrow.left").frame(width: 16)
              }
              .buttonStyle(LabButtonStyle(compact: true)).help(strings("Back"))
            }
            Text(strings("%@ of 4", strings.number(step + 1)))
              .font(.system(size: 10, design: .monospaced)).foregroundStyle(Lab.muted)
            Spacer()
            if step == chapters.count - 1 && canOpenMatrix {
              Button(strings("Open my matrix")) { onFinish(.importMatrix) }
                .buttonStyle(LabButtonStyle(compact: true))
              Button {
                onFinish(.example)
              } label: {
                Label(strings("Try the example"), systemImage: "sparkles")
              }
              .buttonStyle(LabButtonStyle(prominent: true, compact: true))
            } else if step == chapters.count - 1 {
              Button(strings("Back to exploring")) { onFinish(.workspace) }
                .buttonStyle(LabButtonStyle(prominent: true, compact: true))
            } else {
              Button {
                go(to: step + 1)
              } label: {
                Label(strings("Continue"), systemImage: "arrow.right")
              }
              .buttonStyle(LabButtonStyle(prominent: true, compact: true))
              .keyboardShortcut(.defaultAction)
            }
          }.padding(24)
        }.frame(maxWidth: 680, minHeight: 475, maxHeight: 510, alignment: .topLeading)
          .background { ArrivalCard() }
      }.frame(maxWidth: .infinity, maxHeight: .infinity)
      HStack {
        ArrivalFooter(strings: strings)
        Button(strings(step == chapters.count - 1 ? "Explore on my own" : "Skip the tour")) {
          onFinish(.workspace)
        }
        .buttonStyle(.plain).font(.system(size: 11)).foregroundStyle(Lab.secondary)
        .padding(.leading, 22).padding(.top, 16)
      }
    }.padding(.horizontal, 38).padding(.top, 26).padding(.bottom, 20)
      .onExitCommand { onFinish(.workspace) }
  }

  @ViewBuilder private var illustration: some View {
    switch step {
    case 0:
      HStack(spacing: 12) {
        smallCard(
          "cpu", title: "Made for your Mac", caption: "A native home for your research.",
          tint: Lab.accent)
        smallCard(
          "lock.shield", title: "Your data stays here",
          caption: "Matrices and analyses stay local.", tint: Lab.ice)
      }
    case 1:
      HStack(spacing: 22) {
        VStack(spacing: 7) {
          ForEach(0..<4) { row in
            HStack(spacing: 7) {
              Text(verbatim: "T\(row + 1)").font(.system(size: 10, design: .monospaced))
                .foregroundStyle(Lab.muted).frame(width: 23)
              ForEach(0..<8) { column in
                let missing = (row + column) % 7 == 0
                Text(missing ? "?" : "\((row + column / 2) % 3)")
                  .font(.system(size: 11, weight: .medium, design: .monospaced))
                  .foregroundStyle(missing ? Lab.muted : (column % 3 == 0 ? Lab.ice : Lab.accent))
                  .frame(width: 25, height: 25)
                  .background(
                    (missing ? Lab.muted : Lab.accent).opacity(0.07),
                    in: RoundedRectangle(cornerRadius: 5))
              }
            }
          }
        }.accessibilityLabel(strings("Illustration of a morphological matrix"))
          .accessibilityElement(children: .ignore)
        VStack(alignment: .leading, spacing: 10) {
          Label(strings("Taxa"), systemImage: "arrow.down")
          Label(strings("Characters"), systemImage: "arrow.right")
          Text(strings("? means an unknown observation."))
            .foregroundStyle(Lab.muted).lineSpacing(3).fixedSize(horizontal: false, vertical: true)
        }.font(.system(size: 11)).foregroundStyle(Lab.secondary)
      }.padding(18).frame(maxWidth: .infinity, alignment: .leading)
        .background(Lab.background.opacity(0.6), in: RoundedRectangle(cornerRadius: 15))
    case 2:
      HStack(spacing: 24) {
        TourTree().frame(width: 220, height: 115).accessibilityHidden(true)
        VStack(alignment: .leading, spacing: 10) {
          Label(strings("For the bundled example"), systemImage: "lightbulb")
            .font(.system(size: 12, weight: .medium)).foregroundStyle(Lab.ice)
          Text(strings("Choose T20 as the outgroup. Check the remaining settings before starting."))
            .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
        }
      }.padding(22).background(Lab.background.opacity(0.6), in: RoundedRectangle(cornerRadius: 15))
    default:
      HStack(spacing: 10) {
        smallCard(
          "waveform.path", title: "Check the sampling",
          caption: "Useful samples matter more than step counts.", tint: Lab.ice)
        smallCard(
          "point.3.connected.trianglepath.dotted", title: "Explore the trees",
          caption: "Inspect support and export your figures.", tint: Lab.accent)
        smallCard(
          "clock.arrow.circlepath", title: "Keep your place",
          caption: "Find runs and resume saved checkpoints.", tint: Lab.coral)
      }
    }
  }

  private func smallCard(_ symbol: String, title: String, caption: String, tint: Color) -> some View
  {
    VStack(alignment: .leading, spacing: 10) {
      Image(systemName: symbol).font(.system(size: 21, weight: .light)).foregroundStyle(tint)
      Text(strings(title)).font(.system(size: 12, weight: .medium))
      Text(strings(caption)).font(.system(size: 11)).foregroundStyle(Lab.muted).lineSpacing(3)
        .fixedSize(horizontal: false, vertical: true)
      Spacer(minLength: 0)
    }.padding(17).frame(maxWidth: .infinity, minHeight: 136, alignment: .topLeading)
      .background(tint.opacity(0.035), in: RoundedRectangle(cornerRadius: 14))
      .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(tint.opacity(0.09)))
  }

  private func go(to step: Int) {
    withAnimation(comfort.reduceMotion ? nil : .easeInOut(duration: 0.24)) { self.step = step }
  }
}

private struct TourTree: View {
  var body: some View {
    GeometryReader { g in
      let w = g.size.width - 45
      let h = g.size.height
      ZStack(alignment: .topLeading) {
        Path { path in
          path.move(to: CGPoint(x: 8, y: h * 0.5))
          path.addLine(to: CGPoint(x: 30, y: h * 0.5))
          path.move(to: CGPoint(x: 30, y: h * 0.25))
          path.addLine(to: CGPoint(x: 30, y: h * 0.85))
          path.addLine(to: CGPoint(x: w, y: h * 0.85))
          path.move(to: CGPoint(x: 30, y: h * 0.25))
          path.addLine(to: CGPoint(x: w * 0.65, y: h * 0.25))
          path.move(to: CGPoint(x: w * 0.65, y: 8))
          path.addLine(to: CGPoint(x: w * 0.65, y: h * 0.45))
          path.addLine(to: CGPoint(x: w, y: h * 0.45))
          path.move(to: CGPoint(x: w * 0.65, y: 8))
          path.addLine(to: CGPoint(x: w, y: 8))
        }.stroke(
          Lab.accent.opacity(0.7),
          style: StrokeStyle(lineWidth: 1.5, lineCap: .round, lineJoin: .round))
        ForEach(
          Array([("T1", CGFloat(8)), ("T2", h * 0.45), ("T20", h * 0.85)].enumerated()),
          id: \.offset
        ) { index, item in
          Circle().fill(index == 2 ? Lab.ice : Lab.accent).frame(width: 5, height: 5).offset(
            x: w - 2, y: item.1 - 2)
          Text(item.0).font(.system(size: 10, weight: .medium, design: .monospaced))
            .foregroundStyle(index == 2 ? Lab.ice : Lab.secondary).offset(x: w + 10, y: item.1 - 6)
        }
      }
    }
  }
}
