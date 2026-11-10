import Foundation
import ImageIO
import PhotosUI
import Tauri
import UIKit
import UniformTypeIdentifiers

struct Picked: Encodable {
  let path: String
  let name: String?
  let kind: String
  let thumbnailPath: String?
}

struct Cancelled: Encodable {
  let cancelled = true
}

struct Failure: Error {
  let message: String
}

enum Shrinking {
  static let maxSide = 2048
  static let jpegQuality = 0.85
  static let thumbnailSide = 192
  static let thumbnailQuality = 0.7

  static func shrink(_ url: URL, named name: String?) throws -> Picked {
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else {
      throw Failure(message: "the photo cannot be read")
    }
    let png = CGImageSourceGetType(source).map { $0 as String == UTType.png.identifier } ?? false
    let image = try thumbnail(of: source, side: maxSide)
    let target = temporary(png ? "png" : "jpg")
    try write(image, to: target, as: png ? UTType.png : UTType.jpeg, quality: jpegQuality)
    return Picked(
      path: target.path, name: name, kind: png ? "png" : "jpeg", thumbnailPath: small(of: source))
  }

  static func small(of source: CGImageSource) -> String? {
    let target = temporary("jpg")
    do {
      let image = try thumbnail(of: source, side: thumbnailSide)
      try write(image, to: target, as: UTType.jpeg, quality: thumbnailQuality)
      return target.path
    } catch {
      return nil
    }
  }

  static func temporary(_ suffix: String) -> URL {
    URL(fileURLWithPath: NSTemporaryDirectory())
      .appendingPathComponent("\(UUID().uuidString).\(suffix)")
  }

  static func thumbnail(of source: CGImageSource, side: Int) throws -> CGImage {
    let options: [CFString: Any] = [
      kCGImageSourceThumbnailMaxPixelSize: min(side, longestSide(of: source)),
      kCGImageSourceCreateThumbnailWithTransform: true,
      kCGImageSourceCreateThumbnailFromImageAlways: true,
    ]
    guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else {
      throw Failure(message: "the photo cannot be decoded")
    }
    return image
  }

  static func longestSide(of source: CGImageSource) -> Int {
    let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any] ?? [:]
    let width = properties[kCGImagePropertyPixelWidth] as? Int ?? maxSide
    let height = properties[kCGImagePropertyPixelHeight] as? Int ?? maxSide
    return max(width, height)
  }

  static func write(_ image: CGImage, to target: URL, as type: UTType, quality: Double) throws {
    guard
      let destination = CGImageDestinationCreateWithURL(
        target as CFURL, type.identifier as CFString, 1, nil)
    else {
      throw Failure(message: "the photo cannot be written")
    }
    let properties: [CFString: Any] =
      type == .png ? [:] : [kCGImageDestinationLossyCompressionQuality: quality]
    CGImageDestinationAddImage(destination, image, properties as CFDictionary)
    guard CGImageDestinationFinalize(destination) else {
      try? FileManager.default.removeItem(at: target)
      throw Failure(message: "the photo cannot be written")
    }
  }
}

class PhotosPlugin: Plugin, PHPickerViewControllerDelegate, UIAdaptivePresentationControllerDelegate {
  private var pending: Invoke?

  @objc public func pick(_ invoke: Invoke) {
    DispatchQueue.main.async {
      self.present(for: invoke)
    }
  }

  private func present(for invoke: Invoke) {
    guard let root = manager.viewController else {
      invoke.reject("there is no screen to show the photo picker on")
      return
    }
    if pending != nil {
      invoke.reject("the photo picker is already open")
      return
    }
    var configuration = PHPickerConfiguration()
    configuration.filter = .images
    configuration.selectionLimit = 1
    let picker = PHPickerViewController(configuration: configuration)
    picker.delegate = self
    picker.presentationController?.delegate = self
    pending = invoke
    root.present(picker, animated: true)
  }

  func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
    picker.dismiss(animated: true)
    guard let invoke = taken() else {
      return
    }
    guard let provider = results.first?.itemProvider else {
      invoke.resolve(Cancelled())
      return
    }
    load(provider, for: invoke)
  }

  func presentationControllerDidDismiss(_ presentationController: UIPresentationController) {
    taken()?.resolve(Cancelled())
  }

  private func taken() -> Invoke? {
    let invoke = pending
    pending = nil
    return invoke
  }

  private func load(_ provider: NSItemProvider, for invoke: Invoke) {
    let name = provider.suggestedName
    provider.loadFileRepresentation(forTypeIdentifier: UTType.image.identifier) { url, error in
      guard let url = url else {
        invoke.reject("the photo cannot be loaded: \(error?.localizedDescription ?? "no file")")
        return
      }
      do {
        invoke.resolve(try Shrinking.shrink(url, named: name))
      } catch let failure as Failure {
        invoke.reject(failure.message)
      } catch {
        invoke.reject("\(error)")
      }
    }
  }
}

@_cdecl("init_plugin_photos")
func initPlugin() -> Plugin {
  return PhotosPlugin()
}
