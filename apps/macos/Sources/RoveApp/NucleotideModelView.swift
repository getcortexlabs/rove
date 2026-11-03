import SwiftUI

struct NucleotideModelSection: View {
  let strings: AppStrings

  var body: some View {
    LabPanel {
      VStack(alignment: .leading, spacing: 12) {
        Text(strings("Aligned DNA · JC69")).font(.system(size: 13, weight: .semibold))
        Text(
          strings(
            "JC69 treats A, C, G and T equally. All four states remain possible at every site, and constant sites stay in the alignment."
          )
        )
        .font(.system(size: 12)).foregroundStyle(Lab.secondary)
        .fixedSize(horizontal: false, vertical: true)
        Text(
          strings(
            "Import sequences that are already aligned. Ambiguous IUPAC bases represent possible states, not extra nucleotides. Choose a gap policy in the matrix preview."
          )
        )
        .font(.system(size: 11)).foregroundStyle(Lab.secondary)
        .fixedSize(horizontal: false, vertical: true)
      }
    }
  }
}
