import Foundation
import Network
import Tauri

struct WatchArgs: Decodable {
  let channel: Channel
}

struct Path: Encodable {
  let kind: String
  let expensive: Bool

  init(_ path: NWPath) {
    kind = Path.kind(of: path)
    expensive = path.isExpensive
  }

  static func kind(of path: NWPath) -> String {
    if path.status != .satisfied {
      return "none"
    }
    if path.usesInterfaceType(.wifi) {
      return "wifi"
    }
    if path.usesInterfaceType(.cellular) {
      return "cellular"
    }
    return "other"
  }
}

class NetworkPlugin: Plugin {
  private let monitor = NWPathMonitor()
  private let queue = DispatchQueue(label: "cornercase.network")
  private var last: NWPath?

  @objc public func watch(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(WatchArgs.self)
    monitor.pathUpdateHandler = { [weak self] path in
      self?.changed(path, channel: args.channel)
    }
    monitor.start(queue: queue)
    invoke.resolve()
  }

  private func changed(_ path: NWPath, channel: Channel) {
    if path == last {
      return
    }
    last = path
    do {
      try channel.send(Path(path))
    } catch {
      Logger.error("cornercase-mobile: network path not sent: \(error)")
    }
  }
}

@_cdecl("init_plugin_network")
func initPlugin() -> Plugin {
  return NetworkPlugin()
}
