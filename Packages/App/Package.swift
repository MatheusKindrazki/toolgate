// swift-tools-version: 5.10
import PackageDescription
let package = Package(name: "ToolgateApp", platforms: [.macOS(.v14)], products: [.executable(name: "ToolgateApp", targets: ["ToolgateApp"])], targets: [.executableTarget(name: "ToolgateApp")])
