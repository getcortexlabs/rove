import SwiftUI

struct BetaEntryView: View {
  @Bindable var access: BetaAccess
  @Bindable var preferences: AppPreferences
  @State private var email = ""
  @State private var code = ""
  @FocusState private var focus: Field?
  private enum Field { case email, code }
  private var strings: AppStrings { preferences.strings }
  private var waiting: Bool { access.purpose == .waitlist }

  var body: some View {
    VStack(spacing: 0) {
      ArrivalHeader(preferences: preferences, caption: "A LITTLE EARLY. A LOT TO EXPLORE.")
      GeometryReader { geometry in
        ScrollView {
          HStack(spacing: 55) {
            VStack(alignment: .leading, spacing: 20) {
              Eyebrow(text: strings("FOR CURIOUS MINDS"), color: Lab.ice)
              Text(strings("The next discovery\nstarts with hello."))
                .font(Lab.display(46)).tracking(-1.8).fixedSize(horizontal: false, vertical: true)
              Text(
                strings(
                  "We're building Rove with a small group of explorers. Come help shape what comes next."
                )
              )
              .font(.system(size: 14)).foregroundStyle(Lab.secondary).lineSpacing(5)
              .frame(maxWidth: 370, alignment: .leading)
              ExplorerArtwork(strings: strings).scaleEffect(0.78).frame(height: 240)
                .frame(maxWidth: .infinity)
            }.frame(width: 450, alignment: .leading)

            VStack(alignment: .leading, spacing: 24) {
              HStack {
                Label(strings("YOUR FIELD PASS"), systemImage: "ticket")
                  .font(.system(size: 10, weight: .medium, design: .monospaced)).tracking(1.4)
                Spacer()
                Text(verbatim: "01 — 02").font(.system(size: 10, design: .monospaced))
              }.foregroundStyle(Lab.muted)
              VStack(alignment: .leading, spacing: 24) {
                switch access.phase {
                case .checking:
                  ProgressView().controlSize(.small).tint(Lab.accent)
                  Text(strings("Finding your field pass…")).font(Lab.display(27))
                  Text(strings("Just checking your beta access.")).foregroundStyle(Lab.secondary)
                case .waitlisted: confirmation
                case .code: codeForm
                default: emailForm
                }
                if let error = access.error {
                  Label(strings(error), systemImage: "info.circle")
                    .font(.system(size: 12)).foregroundStyle(Lab.coral).lineSpacing(3)
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityAddTraits(.updatesFrequently)
                }
              }
              .padding(30).frame(maxWidth: .infinity, minHeight: 350, alignment: .topLeading)
              .background { ArrivalCard() }
              Label(strings("Your research stays on your Mac."), systemImage: "lock.shield")
                .font(.system(size: 11)).foregroundStyle(Lab.secondary)
                .frame(maxWidth: .infinity)
            }.frame(width: 410)
          }.frame(maxWidth: .infinity, minHeight: geometry.size.height)
        }
      }
      ArrivalFooter(strings: strings)
    }
    .padding(.horizontal, 38).padding(.top, 26).padding(.bottom, 20)
    .onChange(of: access.phase) { _, phase in
      code = ""
      focus = phase == .code ? .code : .email
    }
    .onAppear { email = access.email }
  }

  private var emailForm: some View {
    Group {
      HStack(spacing: 5) {
        modeButton(.access, title: "I have an invitation")
        modeButton(.waitlist, title: "Join the beta list")
      }.padding(4).background(Lab.background.opacity(0.65), in: RoundedRectangle(cornerRadius: 11))
      VStack(alignment: .leading, spacing: 10) {
        Text(strings(waiting ? "Save me a spot." : "A little hello first."))
          .font(Lab.display(30)).tracking(-0.7)
        Text(
          strings(
            waiting
              ? "Leave your email and confirm it's yours. We'll keep your interest on the list."
              : "Use the email on your invitation. We'll send a code to make sure it's you.")
        )
        .font(.system(size: 13)).foregroundStyle(Lab.secondary).lineSpacing(4)
        .fixedSize(horizontal: false, vertical: true)
      }
      VStack(alignment: .leading, spacing: 9) {
        Text(strings("Email address")).font(.system(size: 11, weight: .medium))
        TextField("you@example.com", text: $email)
          .textFieldStyle(.plain).font(.system(size: 14)).padding(14)
          .background(Lab.background.opacity(0.8), in: RoundedRectangle(cornerRadius: 11))
          .overlay(
            RoundedRectangle(cornerRadius: 11).strokeBorder(
              focus == .email ? Lab.accent.opacity(0.65) : Lab.border)
          )
          .focused($focus, equals: .email).accessibilityLabel(strings("Email address"))
          .onSubmit { requestCode() }.disabled(access.busy)
      }
      Button(action: requestCode) {
        HStack {
          Text(strings(waiting ? "Confirm my interest" : "Send my code"))
          Spacer()
          if access.busy {
            ProgressView().controlSize(.mini)
          } else {
            Image(systemName: "arrow.right")
          }
        }
      }.buttonStyle(LabButtonStyle(prominent: true)).disabled(email.isEmpty || access.busy)
      Text(
        strings(
          waiting
            ? "Joining the list doesn't unlock the app. Access is released by invitation."
            : "No invitation yet? You can join the list above.")
      )
      .font(.system(size: 11)).foregroundStyle(Lab.muted).lineSpacing(3)
      .fixedSize(horizontal: false, vertical: true)
      if access.session != nil {
        Button(strings("Check my saved access again")) { Task { await access.refresh() } }
          .buttonStyle(.plain).font(.system(size: 12)).foregroundStyle(Lab.accent)
          .disabled(access.refreshing)
      }
    }
  }

  private var codeForm: some View {
    Group {
      Image(systemName: "envelope.badge.shield.half.filled")
        .font(.system(size: 29, weight: .light)).foregroundStyle(Lab.ice)
        .frame(width: 62, height: 62).background(
          Lab.ice.opacity(0.08), in: RoundedRectangle(cornerRadius: 18))
      VStack(alignment: .leading, spacing: 10) {
        Text(strings("A note in your inbox.")).font(Lab.display(29)).tracking(-0.7)
        Text(
          strings(
            waiting
              ? "Enter the code sent to:" : "If this address is invited, a code will arrive at:")
        )
        .foregroundStyle(Lab.secondary).font(.system(size: 12)).lineSpacing(3)
        Text(access.email).font(.system(size: 13, weight: .medium)).textSelection(.enabled)
          .fixedSize(horizontal: false, vertical: true)
      }
      TextField(strings("8-digit code"), text: $code)
        .textFieldStyle(.plain).font(.system(size: 25, weight: .medium, design: .monospaced))
        .tracking(3).multilineTextAlignment(.center).padding(17)
        .background(Lab.background.opacity(0.8), in: RoundedRectangle(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Lab.accent.opacity(0.5)))
        .focused($focus, equals: .code).accessibilityLabel(strings("8-digit code"))
        .onSubmit { verify() }.disabled(access.busy)
      Button(action: verify) {
        HStack {
          Text(strings(waiting ? "Join the waiting list" : "Let's explore"))
          Spacer()
          if access.busy {
            ProgressView().controlSize(.mini)
          } else {
            Image(systemName: "arrow.up.right")
          }
        }
      }.buttonStyle(LabButtonStyle(prominent: true)).disabled(code.isEmpty || access.busy)
      TimelineView(.periodic(from: .now, by: 1)) { context in
        let seconds = max(0, Int(ceil(access.resendAt.timeIntervalSince(context.date))))
        HStack {
          Button(strings("Change email")) { access.begin(access.purpose) }
          Spacer()
          Button(
            seconds > 0 ? strings("Resend in %@s", strings.number(seconds)) : strings("Resend code")
          ) {
            requestCode(email: access.email)
          }.disabled(seconds > 0 || access.busy)
        }.buttonStyle(.plain).font(.system(size: 11)).foregroundStyle(Lab.secondary)
      }
      Text(strings("Check spam too. The code expires in 10 minutes."))
        .font(.system(size: 11)).foregroundStyle(Lab.muted)
    }
  }

  private var confirmation: some View {
    VStack(alignment: .leading, spacing: 22) {
      Image(systemName: "checkmark.seal")
        .font(.system(size: 34, weight: .light)).foregroundStyle(Lab.ice)
        .frame(width: 72, height: 72).background(
          Lab.ice.opacity(0.08), in: RoundedRectangle(cornerRadius: 22))
      Text(strings("You're on the list.")).font(Lab.display(32)).tracking(-0.8)
      Text(
        strings("Your curiosity is in good company. We've recorded your interest in the Rove beta.")
      )
      .font(.system(size: 13)).foregroundStyle(Lab.secondary).lineSpacing(5)
      Text(access.email).font(.system(size: 12, weight: .medium)).textSelection(.enabled)
      Text(
        strings(
          "Access still needs an invitation. Once your email is approved, come back to sign in.")
      )
      .font(.system(size: 12)).foregroundStyle(Lab.muted).lineSpacing(4)
      Button(strings("Back to sign in")) { access.begin(.access) }
        .buttonStyle(LabButtonStyle(prominent: true))
    }.fixedSize(horizontal: false, vertical: true)
  }

  private func modeButton(_ purpose: BetaPurpose, title: String) -> some View {
    Button {
      access.begin(purpose)
    } label: {
      Text(strings(title)).font(.system(size: 11, weight: .medium))
        .frame(maxWidth: .infinity).padding(.vertical, 10)
        .background(
          access.purpose == purpose ? Lab.raised : .clear, in: RoundedRectangle(cornerRadius: 8)
        )
        .foregroundStyle(access.purpose == purpose ? Lab.ink : Lab.muted)
    }.buttonStyle(.plain).disabled(access.busy)
      .accessibilityAddTraits(access.purpose == purpose ? .isSelected : [])
  }

  private func requestCode() { requestCode(email: email) }
  private func requestCode(email: String) {
    Task {
      await access.request(
        email: email, language: preferences.strings.locale.identifier == "pt_BR" ? "pt-BR" : "en")
    }
  }
  private func verify() { Task { await access.verify(code: code) } }
}

struct ArrivalHeader: View {
  @Bindable var preferences: AppPreferences
  let caption: String
  private var strings: AppStrings { preferences.strings }
  var body: some View {
    HStack(spacing: 11) {
      if let url = Lab.resource("RoveLogo", extension: "png"), let image = NSImage(contentsOf: url)
      {
        Image(nsImage: image).resizable().scaledToFit().frame(width: 37, height: 37)
      }
      Text(verbatim: "Rove").font(Lab.display(24)).tracking(-0.6)
      Text(strings("BETA")).font(.system(size: 9, weight: .medium, design: .monospaced))
        .tracking(1).foregroundStyle(Lab.ice).padding(.horizontal, 8).padding(.vertical, 5)
        .background(Lab.ice.opacity(0.07), in: Capsule()).padding(.leading, 4)
      Spacer()
      Text(strings(caption)).font(.system(size: 9, design: .monospaced))
        .tracking(1.3).foregroundStyle(Lab.muted).padding(.trailing, 18)
      Picker(strings("Language"), selection: $preferences.language) {
        Text("English").tag(AppLanguage.english)
        Text("Português").tag(AppLanguage.portuguese)
        Text(strings("System")).tag(AppLanguage.system)
      }.labelsHidden().pickerStyle(.menu).frame(width: 112).controlSize(.small)
    }.frame(height: 48)
  }
}

struct ArrivalCard: View {
  @Environment(\.visualComfort) private var comfort
  var body: some View {
    RoundedRectangle(cornerRadius: 26)
      .fill(
        LinearGradient(
          colors: [Lab.raised, Lab.surface], startPoint: .topLeading, endPoint: .bottomTrailing)
      )
      .overlay(
        RoundedRectangle(cornerRadius: 26).fill(.ultraThinMaterial).opacity(
          comfort.reduceTransparency ? 0 : 0.15)
      )
      .overlay(
        RoundedRectangle(cornerRadius: 26).strokeBorder(
          LinearGradient(
            colors: [.white.opacity(0.14), Lab.accent.opacity(0.05), .white.opacity(0.04)],
            startPoint: .topLeading, endPoint: .bottomTrailing))
      )
      .shadow(color: .black.opacity(0.17), radius: 24, y: 15)
  }
}

struct ArrivalFooter: View {
  let strings: AppStrings
  var body: some View {
    HStack {
      Text(strings("A project by")).foregroundStyle(Lab.muted)
      Text(verbatim: "Cortex Labs").foregroundStyle(Lab.secondary)
      Spacer()
      Text(strings("Made for curious minds.")).foregroundStyle(Lab.muted)
      Image(systemName: "sparkle").foregroundStyle(Lab.accent)
    }.font(.system(size: 10)).padding(.top, 16)
  }
}
