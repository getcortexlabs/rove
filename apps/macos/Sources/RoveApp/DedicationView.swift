import SwiftUI

struct DedicationView: View {
  @Bindable var preferences: AppPreferences
  var onContinue: () -> Void
  @Environment(\.visualComfort) private var comfort
  @State private var appeared = false
  private var strings: AppStrings { preferences.strings }
  private let paragraphs = [
    "I came into this wanting to make the analyses run faster. You brought your knowledge of biology and research, and our conversations helped shape the project. We just wanted to spend less time waiting. Apparently, that required building an entire app.",
    "Thank you for sharing what you know and for building this with me. There’s a lot here I couldn’t have done on my own.",
    "I really want to see Rove in other people’s hands, helping with research we haven’t even heard of yet. There’s still plenty of work ahead, and I’m excited to keep working on it with you.",
  ]

  var body: some View {
    VStack(spacing: 0) {
      ArrivalHeader(preferences: preferences, caption: "BEFORE WE BEGIN")
      VStack(spacing: 0) {
        ScrollView {
          VStack(alignment: .leading, spacing: 20) {
            HStack(alignment: .center, spacing: 24) {
              Text(strings("Thanks, Thiago."))
                .font(Lab.display(46)).tracking(-1.5)
                .accessibilityAddTraits(.isHeader)
              Spacer(minLength: 0)
              seal
            }
            Text(
              strings(
                "Rove wouldn’t exist without you. I wanted everyone opening the app to know that right from the start."
              )
            )
            .font(.system(size: 16, weight: .medium)).lineSpacing(5)

            VStack(alignment: .leading, spacing: 16) {
              ForEach(paragraphs, id: \.self) { paragraph in
                Text(strings(paragraph))
              }
            }.font(.system(size: 14)).lineSpacing(5).foregroundStyle(Lab.secondary)

            VStack(alignment: .leading, spacing: 10) {
              Text(strings("Let’s keep going.")).font(.system(size: 14, weight: .medium))
              Text(verbatim: "— Rafael")
                .font(.system(size: 23, weight: .medium, design: .serif)).italic()
                .foregroundStyle(Lab.accent)
            }.padding(.top, 3)
          }
          .fixedSize(horizontal: false, vertical: true)
          .textSelection(.enabled)
          .frame(maxWidth: 680, alignment: .leading)
          .padding(.horizontal, 42).padding(.vertical, 32)
          .frame(maxWidth: .infinity)
        }
        Rectangle().fill(Lab.border.opacity(0.5)).frame(height: 1)
        HStack {
          Image(systemName: "heart")
            .font(.system(size: 16, weight: .light)).foregroundStyle(Lab.coral)
            .accessibilityHidden(true)
          Spacer()
          Button(action: onContinue) {
            Label(strings("Continue"), systemImage: "arrow.right")
          }
          .buttonStyle(LabButtonStyle(prominent: true, compact: true))
          .keyboardShortcut(.defaultAction)
        }.padding(.horizontal, 38).padding(.vertical, 18)
      }
      .frame(maxWidth: 840, maxHeight: 650)
      .background { ArrivalCard() }
      .opacity(appeared || comfort.reduceMotion ? 1 : 0)
      .offset(y: appeared || comfort.reduceMotion ? 0 : 8)
      .padding(.top, 22)
      .frame(maxWidth: .infinity, maxHeight: .infinity)
      ArrivalFooter(strings: strings)
    }
    .padding(.horizontal, 38).padding(.top, 26).padding(.bottom, 20)
    .onAppear {
      withAnimation(comfort.reduceMotion ? nil : .easeOut(duration: 0.45)) { appeared = true }
    }
  }

  private var seal: some View {
    ZStack {
      Circle().fill(Lab.accent.opacity(0.06))
      Circle().strokeBorder(Lab.accent.opacity(0.28), lineWidth: 0.75)
      Circle().strokeBorder(Lab.accent.opacity(0.12), lineWidth: 0.75).padding(5)
      Text(verbatim: "TS").font(Lab.display(22, weight: .medium))
        .foregroundStyle(Lab.accent)
    }
    .frame(width: 64, height: 64)
    .rotationEffect(.degrees(-9)).accessibilityHidden(true)
  }
}
