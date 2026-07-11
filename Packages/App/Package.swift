// swift-tools-version: 6.0
import PackageDescription
let package = Package(name: "ToolgateApp", platforms: [.macOS(.v14)], products: [.executable(name: "ToolgateApp", targets: ["ToolgateApp"])], targets: [.executableTarget(name: "ToolgateApp")])
