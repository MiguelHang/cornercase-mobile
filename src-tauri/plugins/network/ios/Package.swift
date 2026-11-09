// swift-tools-version:5.9

import PackageDescription

let package = Package(
  name: "tauri-plugin-network",
  platforms: [
    .iOS(.v15)
  ],
  products: [
    .library(
      name: "tauri-plugin-network",
      type: .static,
      targets: ["tauri-plugin-network"])
  ],
  dependencies: [
    .package(name: "Tauri", path: "../.tauri/tauri-api")
  ],
  targets: [
    .target(
      name: "tauri-plugin-network",
      dependencies: [
        .byName(name: "Tauri")
      ],
      path: "Sources")
  ]
)
