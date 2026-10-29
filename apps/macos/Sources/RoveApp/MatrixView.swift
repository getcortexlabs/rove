import SwiftUI

struct MatrixView: View {
  @Bindable var workspace: Workspace
  private var strings: AppStrings { workspace.strings }
  let report: MatrixReport

  var body: some View {
    VStack(alignment: .leading, spacing: 23) {
      HStack(alignment: .top) {
        VStack(alignment: .leading, spacing: 10) {
          Eyebrow(text: strings("01 / The starting point"), color: Lab.accent)
          Text(strings("Meet your matrix.")).font(Lab.display(35)).tracking(-1)
          Text(
            workspace.isExample
              ? strings("Twenty terminals. Fifty characters. Plenty to be curious about.")
              : workspace.file?.lastPathComponent ?? ""
          )
          .font(.system(size: 13)).foregroundStyle(Lab.secondary)
        }
        Spacer()
        HStack(spacing: 6) {
          Image(systemName: "checkmark.circle.fill")
          Text(strings("Matrix imported"))
        }.font(.system(size: 11, weight: .medium)).foregroundStyle(Lab.accent)
          .padding(.horizontal, 12).padding(.vertical, 8)
          .background(Lab.accent.opacity(0.08), in: Capsule())
      }.padding(.top, 16)

      HStack(spacing: 0) {
        metric(
          "TERMINALS", value: strings.number(report.taxonCount), note: "The taxa in your study")
        metric("CHARACTERS", value: strings.number(report.characterCount), note: "Observed traits")
        metric(
          "PATTERNS", value: report.patternCount.map(strings.number) ?? "—",
          note: "Unique character columns")
        metric(
          "MISSING",
          value: (report.missingPercent / 100).formatted(
            .percent.precision(.fractionLength(1)).locale(strings.locale)),
          note: strings("Unknown cells: %@", strings.number(report.missingCount)))
      }.padding(.vertical, 20)
        .background(Lab.surface, in: RoundedRectangle(cornerRadius: 15))
        .overlay(RoundedRectangle(cornerRadius: 15).strokeBorder(Lab.border.opacity(0.7)))

      VStack(alignment: .leading, spacing: 0) {
        HStack {
          Text(strings("A closer look")).font(.system(size: 13, weight: .semibold))
          Spacer()
          ForEach(Array(report.symbols.prefix(4).enumerated()), id: \.offset) { index, symbol in
            legend(symbol, color: stateColors(index).background)
          }
          legend(strings("? / other"), color: Lab.coral.opacity(0.65))
        }.padding(17)
        Rectangle().fill(Lab.border).frame(height: 1)
        MatrixGrid(report: report, strings: strings).id(report.inputSha256).frame(height: 308)
      }.background(Lab.surface, in: RoundedRectangle(cornerRadius: 16))
        .overlay(RoundedRectangle(cornerRadius: 16).strokeBorder(Lab.border))
        .clipShape(RoundedRectangle(cornerRadius: 16))

      if report.gapCount > 0 {
        Toggle(isOn: $workspace.settings.gapsAsMissing) {
          VStack(alignment: .leading, spacing: 4) {
            Text(strings("Treat gaps as missing data (%@)", strings.number(report.gapCount))).font(
              .system(size: 12, weight: .medium))
            Text(strings("Only choose this if gaps mean an unknown observation in your matrix."))
              .font(.system(size: 11)).foregroundStyle(Lab.secondary)
          }
        }.toggleStyle(.switch).tint(Lab.accent)
          .onChange(of: workspace.settings.gapsAsMissing) { workspace.reloadGapPolicy() }
      }
      if let issue = report.samplerError {
        DiagnosticMessage(
          title: strings("This matrix cannot be sampled with the current model."), details: issue,
          strings: strings)
      } else {
        HStack(alignment: .top, spacing: 9) {
          Image(systemName: "info.circle").foregroundStyle(Lab.lilac)
          Text(
            strings("Repeated columns share a calculation. Every character still counts.")
          )
          .foregroundStyle(Lab.secondary)
        }.font(.system(size: 11)).padding(.horizontal, 3)
      }
      if report.uncertainCount + report.polymorphicCount > 0 {
        Text(
          strings(
            "Uncertain cells: %@ · polymorphic cells: %@", strings.number(report.uncertainCount),
            strings.number(report.polymorphicCount))
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary)
      }
    }
  }

  private func metric(_ label: String, value: String, note: String) -> some View {
    VStack(alignment: .leading, spacing: 8) {
      Eyebrow(text: strings(label))
      Text(value).font(Lab.display(30, weight: .semibold)).foregroundStyle(Lab.ink)
      Text(strings(note)).font(.system(size: 10)).foregroundStyle(Lab.secondary)
    }.frame(maxWidth: .infinity, alignment: .leading).padding(.leading, 22)
  }

  private func legend(_ label: String, color: Color) -> some View {
    HStack(spacing: 5) {
      RoundedRectangle(cornerRadius: 3).fill(color).frame(width: 9, height: 9)
      Text(label).font(.system(size: 10)).foregroundStyle(Lab.secondary)
    }.padding(.leading, 8)
  }
}

private struct MatrixGrid: View {
  let report: MatrixReport
  let strings: AppStrings
  @State private var selected: (row: Int, column: Int)?
  private let cell: CGFloat = 25
  private let label: CGFloat = 95
  private let header: CGFloat = 34

  var body: some View {
    VStack(spacing: 0) {
      ScrollView([.horizontal, .vertical]) {
        Canvas { context, _ in
          for column in report.characterLabels.indices {
            let x = label + CGFloat(column) * cell
            context.draw(
              Text(String(column + 1)).font(.system(size: 9, design: .monospaced))
                .foregroundStyle(Lab.muted), at: CGPoint(x: x + cell / 2, y: header / 2))
          }
          for row in report.taxa.indices {
            let y = header + CGFloat(row) * cell
            let taxon = report.taxa[row]
            context.draw(
              Text(taxon.count > 13 ? String(taxon.prefix(12)) + "…" : taxon)
                .font(.system(size: 10, weight: .medium, design: .monospaced))
                .foregroundStyle(Lab.secondary), at: CGPoint(x: 14, y: y + cell / 2),
              anchor: .leading)
            for column in report.cells[row].indices {
              let value = report.cells[row][column]
              let rect = CGRect(
                x: label + CGFloat(column) * cell + 2, y: y + 2,
                width: cell - 4, height: cell - 4)
              let path = Path(roundedRect: rect, cornerRadius: 4)
              let colors =
                report.symbols.firstIndex(of: value).map(stateColors)
                ?? (background: Lab.coral.opacity(0.18), text: Lab.coral)
              context.fill(path, with: .color(colors.background))
              context.draw(
                Text(value).font(
                  .system(size: value.count > 1 ? 7 : 10, weight: .medium, design: .monospaced)
                )
                .foregroundStyle(colors.text),
                at: CGPoint(x: rect.midX, y: rect.midY))
              if selected?.row == row && selected?.column == column {
                context.stroke(path, with: .color(Lab.ink), lineWidth: 1.5)
              }
            }
          }
        }
        .frame(
          width: label + CGFloat(report.characterLabels.count) * cell + 12,
          height: header + CGFloat(report.taxa.count) * cell + 12
        )
        .contentShape(Rectangle())
        .onTapGesture { location in
          let row = Int(floor((location.y - header) / cell))
          let column = Int(floor((location.x - label) / cell))
          if report.taxa.indices.contains(row), report.characterLabels.indices.contains(column) {
            selected = (row, column)
          }
        }
        .accessibilityLabel(strings("Morphological matrix preview"))
        .accessibilityChildren {
          ForEach(report.taxa.indices, id: \.self) { row in
            Text("\(report.taxa[row]): \(report.cells[row].joined(separator: ", "))")
          }
        }
      }
      HStack {
        if let selected {
          let name =
            report.characterLabels[selected.column]
            ?? strings("Character %@", strings.number(selected.column + 1))
          Text(
            "\(report.taxa[selected.row])  ·  \(name)  ·  \(report.cells[selected.row][selected.column])"
          )
        } else {
          Text(strings("Click a cell to inspect it. Scroll to explore."))
        }
        Spacer()
        Text(
          strings(
            "%@ × %@ shown", strings.number(report.taxa.count),
            strings.number(report.characterLabels.count)))
      }.font(.system(size: 10)).foregroundStyle(Lab.secondary).padding(.horizontal, 15).padding(
        .vertical, 10
      )
      .background(Lab.raised.opacity(0.55))
    }
  }
}

private func stateColors(_ index: Int) -> (background: Color, text: Color) {
  switch index {
  case 0: (Lab.ice.opacity(0.13), Lab.ice)
  case 1: (Lab.lilac.opacity(0.82), Lab.background)
  case 2: (Color(hex: 0xE8BE99).opacity(0.65), Lab.background)
  default: (Lab.coral.opacity(0.5), Lab.background)
  }
}

struct InlineMessage: View {
  let text: String
  var isError = false
  var body: some View {
    HStack(alignment: .top, spacing: 9) {
      Image(systemName: isError ? "exclamationmark.circle" : "checkmark.circle")
      Text(text).textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
      Spacer(minLength: 0)
    }.font(.system(size: 12)).foregroundStyle(isError ? Lab.coral : Lab.accent)
      .padding(14).background(
        (isError ? Lab.coral : Lab.accent).opacity(0.07), in: RoundedRectangle(cornerRadius: 10)
      )
      .accessibilityElement(children: .combine)
  }
}
