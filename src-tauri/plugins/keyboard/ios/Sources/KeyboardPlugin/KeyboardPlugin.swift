import Tauri
import UIKit
import WebKit

final class Bare: NSObject {
  @objc var inputAccessoryView: UIView? { nil }
}

enum AccessoryBar {
  static let suffix = "_WithoutAccessoryBar"

  static func remove(from webview: WKWebView) {
    guard let content = webview.scrollView.subviews.first(where: isContent),
      let bare = bareClass(of: content)
    else { return }
    object_setClass(content, bare)
  }

  static func isContent(_ view: UIView) -> Bool {
    NSStringFromClass(type(of: view)).hasPrefix("WKContent")
  }

  static func bareClass(of view: UIView) -> AnyClass? {
    guard let base = object_getClass(view) else { return nil }
    let name = NSStringFromClass(base) + suffix
    if let made = NSClassFromString(name) { return made }
    return make(name, under: base)
  }

  static func make(_ name: String, under base: AnyClass) -> AnyClass? {
    let getter = #selector(getter: Bare.inputAccessoryView)
    guard let made = objc_allocateClassPair(base, name, 0),
      let method = class_getInstanceMethod(Bare.self, getter)
    else { return nil }
    class_addMethod(made, getter, method_getImplementation(method), method_getTypeEncoding(method))
    objc_registerClassPair(made)
    return made
  }
}

enum Dismissal {
  static let blur = "document.activeElement?.blur?.()"
  static let asked = "document.documentElement.dataset.keyboard === 'follows'"

  static func set(_ mode: UIScrollView.KeyboardDismissMode, in view: UIView) {
    if let scrolled = view as? UIScrollView {
      scrolled.keyboardDismissMode = mode
    }
    view.subviews.forEach { set(mode, in: $0) }
  }
}

final class ScrollLock {
  var held: [NSKeyValueObservation] = []

  func hold(under view: UIView) {
    guard held.isEmpty else { return }
    held = Self.dragged(under: view).map(Self.pin)
  }

  func release() {
    held.forEach { $0.invalidate() }
    held = []
  }

  static func pin(_ scrolled: UIScrollView) -> NSKeyValueObservation {
    let offset = scrolled.contentOffset
    return scrolled.observe(\.contentOffset) { scrolled, _ in
      if scrolled.contentOffset != offset { scrolled.contentOffset = offset }
    }
  }

  static func dragged(under view: UIView) -> [UIScrollView] {
    let own = (view as? UIScrollView).map { $0.isDragging ? [$0] : [] } ?? []
    return own + view.subviews.flatMap(dragged)
  }
}

final class KeyboardTop: UIView {
  static let event = "keyboardtop"
  static let slack: CGFloat = 1

  weak var webview: WKWebView?
  var told: CGFloat?
  var rest: CGFloat?
  var follows = false
  let lock = ScrollLock()

  override func layoutSubviews() {
    super.layoutSubviews()
    tell(bounds.height)
    holdWhileLeaving()
  }

  func holdWhileLeaving() {
    guard follows, let rest, bounds.height > rest + Self.slack else { return lock.release() }
    webview.map(lock.hold)
  }

  func shown() {
    rest = bounds.height
    lock.release()
    tell(nil)
    webview?.evaluateJavaScript(Dismissal.asked) { [weak self] answer, _ in
      guard let self, self.rest != nil else { return }
      self.follow(answer as? Bool ?? false)
    }
  }

  func follow(_ asked: Bool) {
    follows = asked
    webview.map { Dismissal.set(asked ? .interactive : .none, in: $0) }
  }

  func gone() {
    if follows { webview?.evaluateJavaScript(Dismissal.blur) }
    follow(false)
    rest = nil
    lock.release()
  }

  func tell(_ top: CGFloat?) {
    guard top != told else { return }
    told = top
    let detail = top.map { "\(Double($0))" } ?? "null"
    webview?.evaluateJavaScript("dispatchEvent(new CustomEvent('\(Self.event)', { detail: \(detail) }))")
  }

  static func track(_ webview: WKWebView) -> KeyboardTop {
    let top = KeyboardTop()
    top.webview = webview
    top.isUserInteractionEnabled = false
    top.translatesAutoresizingMaskIntoConstraints = false
    webview.addSubview(top)
    NSLayoutConstraint.activate([
      top.topAnchor.constraint(equalTo: webview.topAnchor),
      top.bottomAnchor.constraint(equalTo: webview.keyboardLayoutGuide.topAnchor),
      top.leadingAnchor.constraint(equalTo: webview.leadingAnchor),
      top.widthAnchor.constraint(equalToConstant: 0),
    ])
    return top
  }
}

class KeyboardPlugin: Plugin {
  var observers: [NSObjectProtocol] = []

  override func load(webview: WKWebView) {
    AccessoryBar.remove(from: webview)
    let top = KeyboardTop.track(webview)
    observers = [
      observe(UIResponder.keyboardDidShowNotification) { [weak top] in top?.shown() },
      observe(UIResponder.keyboardDidHideNotification) { [weak top] in top?.gone() },
    ]
  }

  func observe(_ name: Notification.Name, then: @escaping () -> Void) -> NSObjectProtocol {
    NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) { _ in then() }
  }
}

@_cdecl("init_plugin_keyboard")
func initPlugin() -> Plugin {
  return KeyboardPlugin()
}
