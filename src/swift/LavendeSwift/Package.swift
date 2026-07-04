// swift-tools-version: 6.3
import PackageDescription

let package = Package(
    name: "Lavende",
    platforms: [
        .macOS(.v10_15), .iOS(.v13)
    ],
    products: [
        .library(
            name: "Lavende",
            targets: ["Lavende"]),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "Lavende",
            dependencies: ["LavendeSwift"],
            path: "Sources/Lavende",
            swiftSettings: [
                .swiftLanguageMode(.v5)
            ]
        ),
        .binaryTarget(
            name: "LavendeSwift",
            path: "LavendeSwift.xcframework"
        )
    ]
)
