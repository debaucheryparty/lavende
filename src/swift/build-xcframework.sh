#!/bin/bash
set -e

# Change directory to the script's location
cd "$(dirname "$0")"

# Variables
CRATE_NAME="lavende_swift"
FRAMEWORK_NAME="LavendeSwift"

echo "Checking required Rust targets..."
rustup target add aarch64-apple-darwin
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim
rustup target add x86_64-apple-ios

# Workaround for audiopus_sys failing with newer CMake versions
cat << 'EOF' > cmake_wrapper.sh
#!/bin/bash
if [[ "$*" == *"--build"* ]]; then
  cmake "$@"
else
  cmake "$@" -DCMAKE_POLICY_VERSION_MINIMUM=3.5
fi
EOF
chmod +x cmake_wrapper.sh
export CMAKE="$(pwd)/cmake_wrapper.sh"

echo "Building Rust library for macOS..."
cargo build --release --target aarch64-apple-darwin

echo "Building Rust library for iOS Device..."
cargo build --release --target aarch64-apple-ios

echo "Building Rust library for iOS Simulator..."
cargo build --release --target aarch64-apple-ios-sim
cargo build --release --target x86_64-apple-ios

# Create output directories
rm -rf build
mkdir -p build/macos
mkdir -p build/ios
mkdir -p build/ios-sim

# Copy macOS library
cp ../../target/aarch64-apple-darwin/release/lib${CRATE_NAME}.a build/macos/lib${CRATE_NAME}.a

# Copy iOS Device library
cp ../../target/aarch64-apple-ios/release/lib${CRATE_NAME}.a build/ios/lib${CRATE_NAME}.a

# Create fat binary for iOS Simulator
lipo -create -output build/ios-sim/lib${CRATE_NAME}.a \
    ../../target/aarch64-apple-ios-sim/release/lib${CRATE_NAME}.a \
    ../../target/x86_64-apple-ios/release/lib${CRATE_NAME}.a

echo "Generating Swift bindings..."
mkdir -p build/swift

# Compile the uniffi-bindgen binary
cargo build --bin uniffi-bindgen --release

# Run uniffi-bindgen using the macOS dynamic library to generate the Swift code
# (We need a dylib or cdylib to extract the uniffi metadata)
cargo run --bin uniffi-bindgen --release -- generate \
    --library ../../target/aarch64-apple-darwin/release/lib${CRATE_NAME}.dylib \
    --language swift \
    --out-dir build/swift

echo "Creating XCFramework..."
# Create a modulemap
cat <<EOF > build/swift/module.modulemap
module ${FRAMEWORK_NAME} {
    header "${CRATE_NAME}FFI.h"
    export *
}
EOF

# Organize headers
mkdir -p build/headers
cp build/swift/*.h build/headers/
cp build/swift/module.modulemap build/headers/

# Remove old xcframework if exists
rm -rf build/${FRAMEWORK_NAME}.xcframework

xcodebuild -create-xcframework \
    -library build/macos/lib${CRATE_NAME}.a \
    -headers build/headers \
    -library build/ios/lib${CRATE_NAME}.a \
    -headers build/headers \
    -library build/ios-sim/lib${CRATE_NAME}.a \
    -headers build/headers \
    -output build/${FRAMEWORK_NAME}.xcframework

echo "Copying into Swift Package directory..."
rm -rf LavendeSwift/LavendeSwift.xcframework
cp -r build/${FRAMEWORK_NAME}.xcframework LavendeSwift/
mkdir -p LavendeSwift/Sources/Lavende
cp build/swift/lavende_swift.swift LavendeSwift/Sources/Lavende/lavende_swift.swift

# Inject the module import so the SPM target can see the C FFI functions
sed -i '' '1i\
import LavendeSwift
' LavendeSwift/Sources/Lavende/lavende_swift.swift

echo "Done! The XCFramework is ready at src/swift/LavendeSwift/LavendeSwift.xcframework"
