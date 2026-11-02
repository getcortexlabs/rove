import AppKit
import Combine
import Sparkle

/// Engine children can outlive their window. Count them at the process boundary
/// so installing an update cannot interrupt a calculation in another tool.
final class EngineActivity: @unchecked Sendable {
  static let shared = EngineActivity()
  static let changed = Notification.Name("RoveEngineActivityChanged")
  private let lock = NSLock()
  private var count = 0
  var isActive: Bool { lock.withLock { count > 0 } }
  func begin() {
    lock.withLock { count += 1 }
    NotificationCenter.default.post(name: Self.changed, object: nil)
  }
  func end() {
    lock.withLock {
      count -= 1
      precondition(count >= 0)
    }
    NotificationCenter.default.post(name: Self.changed, object: nil)
  }
}

/// Installation resumes only after another explicit click. Finishing a long
/// analysis must not make its results disappear behind an unexpected relaunch.
@MainActor
final class UpdateInstallation {
  private let busy: () -> Bool
  private var pending: (() -> Void)?
  var isPending: Bool { pending != nil }
  var canResume: Bool { isPending && !busy() }
  init(busy: @escaping () -> Bool) { self.busy = busy }
  func postpone(_ install: @escaping () -> Void) -> Bool {
    guard busy() else { return false }
    pending = install
    return true
  }
  func resume() {
    guard canResume else { return }
    let install = pending
    pending = nil
    install?()
  }
  func cancel() { pending = nil }
}

@MainActor
final class AppUpdater: NSObject, ObservableObject, SPUUpdaterDelegate {
  @Published private(set) var canCheck = false
  @Published private(set) var automaticChecks = false
  @Published private(set) var automaticDownloads = false
  @Published private(set) var installationPending = false
  @Published private(set) var workActive = false
  @Published private(set) var startupError: String?
  private(set) var installationRequested = false
  private var controller: SPUStandardUpdaterController?
  private var observations: Set<AnyCancellable> = []
  private let busy: () -> Bool
  private let strings: () -> AppStrings
  private let installation: UpdateInstallation
  var configured: Bool { controller != nil }

  static func configured(_ info: [String: Any]) -> Bool {
    guard info["RoveUpdatesEnabled"] as? Bool == true,
      let feed = info["SUFeedURL"] as? String, let url = URL(string: feed),
      url.scheme == "https", url.host != nil, url.user == nil, url.password == nil,
      let key = info["SUPublicEDKey"] as? String, Data(base64Encoded: key)?.count == 32
    else { return false }
    return true
  }

  init(bundle: Bundle = .main, busy: @escaping () -> Bool, strings: @escaping () -> AppStrings) {
    self.busy = busy
    self.strings = strings
    installation = UpdateInstallation(busy: busy)
    super.init()
    guard Self.configured(bundle.infoDictionary ?? [:]) else { return }
    let controller = SPUStandardUpdaterController(
      startingUpdater: false, updaterDelegate: self, userDriverDelegate: nil)
    self.controller = controller
    controller.updater.publisher(for: \.canCheckForUpdates).assign(to: &$canCheck)
    controller.updater.publisher(for: \.automaticallyChecksForUpdates).assign(to: &$automaticChecks)
    controller.updater.publisher(for: \.automaticallyDownloadsUpdates).assign(
      to: &$automaticDownloads)
    NotificationCenter.default.publisher(for: EngineActivity.changed)
      .sink { [weak self] _ in Task { @MainActor in self?.workDidChange() } }
      .store(in: &observations)
    do { try controller.updater.start() } catch { startupError = error.localizedDescription }
    workDidChange()
  }

  func check() { controller?.checkForUpdates(nil) }
  func setAutomaticChecks(_ value: Bool) {
    controller?.updater.automaticallyChecksForUpdates = value
  }
  func setAutomaticDownloads(_ value: Bool) {
    controller?.updater.automaticallyDownloadsUpdates = value
  }
  func workDidChange() { workActive = busy() }
  func install() {
    installation.resume()
    installationPending = installation.isPending
  }

  func updater(_ updater: SPUUpdater, mayPerform updateCheck: SPUUpdateCheck) throws {
    guard !busy() else {
      throw NSError(
        domain: "com.getcortexlabs.rove.updates", code: 1,
        userInfo: [
          NSLocalizedDescriptionKey: strings()(
            "Rove will check for updates when your work has finished.")
        ])
    }
  }

  func updater(_ updater: SPUUpdater, willInstallUpdate item: SUAppcastItem) {
    installationRequested = true
  }

  func updater(
    _ updater: SPUUpdater, shouldPostponeRelaunchForUpdate item: SUAppcastItem,
    untilInvokingBlock installHandler: @escaping () -> Void
  ) -> Bool {
    let postponed = installation.postpone(installHandler)
    installationPending = installation.isPending
    workDidChange()
    return postponed
  }

  func updater(_ updater: SPUUpdater, didAbortWithError error: any Error) {
    installationRequested = false
    installation.cancel()
    installationPending = false
  }
}
