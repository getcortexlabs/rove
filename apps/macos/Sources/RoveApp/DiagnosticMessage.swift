import SwiftUI

struct DiagnosticMessage: View {
  let title: String
  let details: String?
  let strings: AppStrings

  var body: some View {
    VStack(alignment: .leading, spacing: 9) {
      InlineMessage(text: title, isError: true)
      if let details, !details.isEmpty {
        DisclosureGroup(strings("Technical details")) {
          Text(verbatim: details).font(.system(size: 11, design: .monospaced))
            .foregroundStyle(Lab.secondary).textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading).padding(.top, 6)
        }.font(.system(size: 11)).foregroundStyle(Lab.secondary).padding(.horizontal, 4)
      }
    }
  }
}
