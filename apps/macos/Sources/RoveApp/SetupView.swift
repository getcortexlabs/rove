import SwiftUI

struct SetupView: View {
  @Bindable var workspace: Workspace

  var body: some View {
    VStack(alignment: .leading, spacing: 24) {
      VStack(alignment: .leading, spacing: 10) {
        Eyebrow(text: "02 / A few ground rules", color: Lab.lime)
        Text("How should we explore?").font(Lab.display(35)).tracking(-1)
        Text("Choose the assumptions behind your analysis.")
          .font(.system(size: 13)).foregroundStyle(Lab.secondary)
      }.padding(.top, 16)

      VStack(alignment: .leading, spacing: 12) {
        HStack {
          Text("Evolutionary clock").font(.system(size: 13, weight: .semibold))
          Spacer()
          help(
            "The clock links branch duration to expected character changes. These models have no fossil ages or calendar dates."
          )
        }
        HStack(spacing: 14) {
          clockCard(
            "strict", title: "One shared pace", subtitle: "Strict clock", icon: "metronome",
            description:
              "Every branch evolves at the same rate. One rate category across characters.")
          clockCard(
            "reference", title: "Room to wander", subtitle: "Relaxed clock + gamma",
            icon: "point.topleft.down.to.point.bottomright.curvepath",
            description:
              "Rates can vary across branches. Four gamma categories account for variation across characters."
          )
        }
      }

      LabPanel {
        VStack(alignment: .leading, spacing: 15) {
          HStack {
            VStack(alignment: .leading, spacing: 6) {
              Text("Which characters did you include?").font(.system(size: 13, weight: .semibold))
              Text("Match the model to how your matrix was collected.")
                .font(.system(size: 11)).foregroundStyle(Lab.secondary)
            }
            Spacer()
            Picker("Character selection", selection: $workspace.settings.conditioning) {
              Text("Variable only · Mkv").tag("variable")
              Text("All characters · Mk").tag("none")
            }.labelsHidden().frame(width: 200).tint(Lab.lime)
          }
          Text(
            workspace.settings.conditioning == "variable"
              ? "Use Mkv when constant characters were excluded. Missing observations can make a character's variability impossible to establish."
              : "Use Mk when your sampling includes constant as well as variable characters. The matrix can contain both."
          )
          .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
          if workspace.settings.conditioning == "variable",
            let issue = workspace.report?.variableError
          {
            InlineMessage(text: issue, isError: true)
          }
        }
      }

      LabPanel {
        VStack(alignment: .leading, spacing: 19) {
          Text("The exploration plan").font(.system(size: 13, weight: .semibold))
          HStack(alignment: .top, spacing: 25) {
            VStack(alignment: .leading, spacing: 9) {
              Eyebrow(text: "Chain length")
              Picker("Chain length", selection: $workspace.settings.steps) {
                Text("1 million steps").tag(1_000_000)
                Text("10 million steps").tag(10_000_000)
                Text("100 million steps").tag(100_000_000)
              }.labelsHidden().frame(maxWidth: .infinity)
              Text("More steps allow more exploration.").font(.system(size: 10)).foregroundStyle(
                Lab.muted)
            }
            VStack(alignment: .leading, spacing: 9) {
              Eyebrow(text: "Save a sample every")
              Picker("Sampling interval", selection: $workspace.settings.sampleEvery) {
                Text("100 steps").tag(100)
                Text("1,000 steps").tag(1_000)
                Text("10,000 steps").tag(10_000)
              }.labelsHidden().frame(maxWidth: .infinity)
              Text("Controls how often a state is recorded.").font(.system(size: 10))
                .foregroundStyle(Lab.muted)
            }
            VStack(alignment: .leading, spacing: 9) {
              Eyebrow(text: "Random seed")
              TextField("Seed", text: $workspace.settings.seed)
                .textFieldStyle(.roundedBorder).font(.system(size: 12, design: .monospaced))
              Text("A repeatable starting point.").font(.system(size: 10)).foregroundStyle(
                Lab.muted)
            }.frame(width: 155)
          }
          Text(
            "Chain length alone doesn't establish convergence. Independent runs and sampling diagnostics are needed to assess an analysis."
          )
          .font(.system(size: 11)).foregroundStyle(Lab.secondary).lineSpacing(3)
        }
      }
      if let error = workspace.settings.validationError {
        InlineMessage(text: error, isError: true)
      }
    }
  }

  private func clockCard(
    _ value: String, title: String, subtitle: String, icon: String, description: String
  ) -> some View {
    let selected = workspace.settings.clock == value
    return Button {
      workspace.settings.clock = value
    } label: {
      VStack(alignment: .leading, spacing: 12) {
        HStack {
          Image(systemName: icon).font(.system(size: 22, weight: .light)).foregroundStyle(
            selected ? Lab.lime : Lab.lilac)
          Spacer()
          Image(systemName: selected ? "checkmark.circle.fill" : "circle")
            .foregroundStyle(selected ? Lab.lime : Lab.border)
        }
        VStack(alignment: .leading, spacing: 5) {
          Text(title).font(Lab.display(21, weight: .semibold)).foregroundStyle(Lab.ink)
          Text(subtitle).font(.system(size: 11, weight: .medium)).foregroundStyle(
            selected ? Lab.lime : Lab.secondary)
        }
        Text(description).font(.system(size: 12)).foregroundStyle(Lab.secondary)
          .lineSpacing(4).fixedSize(horizontal: false, vertical: true)
      }.frame(maxWidth: .infinity, minHeight: 155, alignment: .topLeading).padding(21)
        .background(
          selected ? Lab.lime.opacity(0.045) : Lab.surface, in: RoundedRectangle(cornerRadius: 16)
        )
        .overlay(
          RoundedRectangle(cornerRadius: 16).strokeBorder(
            selected ? Lab.lime.opacity(0.65) : Lab.border))
    }.buttonStyle(.plain).accessibilityLabel("\(subtitle). \(description)")
      .accessibilityAddTraits(selected ? .isSelected : [])
  }

  private func help(_ message: String) -> some View {
    Image(systemName: "questionmark.circle").foregroundStyle(Lab.muted).help(message)
      .accessibilityLabel(message)
  }
}

struct ReviewView: View {
  @Bindable var workspace: Workspace
  let report: MatrixReport

  var body: some View {
    VStack(alignment: .leading, spacing: 24) {
      VStack(alignment: .leading, spacing: 10) {
        Eyebrow(text: "03 / Check your bearings", color: Lab.lime)
        Text("A good place to begin.").font(Lab.display(35)).tracking(-1)
        Text("Your matrix and assumptions, all in one place.").font(.system(size: 13))
          .foregroundStyle(Lab.secondary)
      }.padding(.top, 16)

      LabPanel {
        VStack(spacing: 0) {
          HStack(spacing: 13) {
            Image(systemName: "square.grid.3x3.fill").font(.system(size: 25)).foregroundStyle(
              Lab.lilac)
            VStack(alignment: .leading, spacing: 5) {
              Text(workspace.file?.lastPathComponent ?? "Matrix").font(
                .system(size: 14, weight: .semibold))
              Text(
                "\(report.taxonCount) terminals · \(report.characterCount) characters · \(report.patternCount ?? 0) patterns"
              )
              .font(.system(size: 11)).foregroundStyle(Lab.secondary)
            }
            Spacer()
            Text("NEXUS").font(.system(size: 10, design: .monospaced)).foregroundStyle(Lab.muted)
          }.padding(.bottom, 22)
          rule
          row(
            "Character model",
            workspace.settings.conditioning == "variable"
              ? "Binary Mkv · variable characters" : "Binary Mk · all characters")
          row(
            "Evolutionary clock",
            workspace.settings.clock == "reference"
              ? "Lognormal relaxed clock · 4 gamma categories" : "Strict clock · 1 rate category")
          row("Tree prior", "Birth–death · all tips at height zero")
          row("Groups constrained", "None")
          rule
          row(
            "Chain",
            "\(workspace.settings.steps.formatted()) steps · seed \(workspace.settings.seed)")
          row("Sampling", "Every \(workspace.settings.sampleEvery.formatted()) steps")
          row("Gaps", workspace.settings.gapsAsMissing ? "Treat as missing" : "Reject")
        }
      }

      HStack(alignment: .top, spacing: 13) {
        Image(systemName: "leaf").font(.system(size: 23)).foregroundStyle(Lab.lime)
        VStack(alignment: .leading, spacing: 8) {
          Text("Ready to keep exploring.").font(Lab.display(20, weight: .semibold))
          Text(
            "Save a record of this setup, or copy a command to run it in Terminal. No analysis has been run in this window."
          )
          .font(.system(size: 12)).foregroundStyle(Lab.secondary).lineSpacing(4)
        }
      }.padding(21).frame(maxWidth: .infinity, alignment: .leading)
        .background(Lab.lime.opacity(0.045), in: RoundedRectangle(cornerRadius: 15))
      if let notice = workspace.notice { InlineMessage(text: notice) }
      HStack(spacing: 12) {
        Button(action: workspace.copyCommand) {
          Label("Copy terminal command", systemImage: "terminal")
        }
        .buttonStyle(LabButtonStyle(compact: true))
        if let saved = workspace.savedURL {
          Button {
            NSWorkspace.shared.activateFileViewerSelecting([saved])
          } label: {
            Label("Show saved setup", systemImage: "folder")
          }.buttonStyle(LabButtonStyle(compact: true))
        }
        Spacer()
        Text("A setup file references your matrix; it doesn't copy it.")
          .font(.system(size: 10)).foregroundStyle(Lab.muted).frame(maxWidth: 215)
      }
    }
  }

  private var rule: some View {
    Rectangle().fill(Lab.border).frame(height: 1).padding(.vertical, 6)
  }
  private func row(_ label: String, _ value: String) -> some View {
    HStack(alignment: .top) {
      Text(label).foregroundStyle(Lab.secondary)
      Spacer(minLength: 20)
      Text(value).foregroundStyle(Lab.ink).multilineTextAlignment(.trailing)
    }.font(.system(size: 12)).padding(.vertical, 11)
  }
}
