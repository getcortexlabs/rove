// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "Rove",
  platforms: [.macOS(.v14)],
  products: [.executable(name: "Rove", targets: ["RoveApp"])],
  targets: [
    .executableTarget(name: "RoveApp", resources: [.copy("Resources")]),
    .testTarget(name: "RoveAppTests", dependencies: ["RoveApp"]),
  ]
)
