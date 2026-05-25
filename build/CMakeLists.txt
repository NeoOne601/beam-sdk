# CMakeLists.txt — Beam SDK cross-platform build
# Targets: Android (AArch64/ARMv7), iOS (arm64), WASM (Emscripten)
#
# Usage:
#   Android:   cmake -DCMAKE_TOOLCHAIN_FILE=$NDK/build/cmake/android.toolchain.cmake \
#                    -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=android-24 .
#   iOS:       cmake -DCMAKE_TOOLCHAIN_FILE=ios.toolchain.cmake -DPLATFORM=OS64 .
#   WASM:      emcmake cmake -DBEAM_TARGET=WASM .

cmake_minimum_required(VERSION 3.22)
project(beam_sdk VERSION 0.1.0 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/..")
include_directories("${REPO_ROOT}/include")

# ─── Platform detection ──────────────────────────────────────────────────────

if (DEFINED EMSCRIPTEN)
    set(BEAM_TARGET "WASM" CACHE STRING "Beam build target")
elseif (ANDROID)
    set(BEAM_TARGET "Android" CACHE STRING "Beam build target")
elseif (IOS OR CMAKE_SYSTEM_NAME STREQUAL "iOS")
    set(BEAM_TARGET "iOS" CACHE STRING "Beam build target")
else()
    set(BEAM_TARGET "Host" CACHE STRING "Beam build target")
endif()

message(STATUS "Beam target: ${BEAM_TARGET}")

# ─── Rust core (cargo build) ─────────────────────────────────────────────────

set(RUST_CORE_DIR "${REPO_ROOT}/core")
set(RUST_LIB_NAME "beam_core")

if (BEAM_TARGET STREQUAL "Android")
    if (ANDROID_ABI STREQUAL "arm64-v8a")
        set(RUST_TARGET "aarch64-linux-android")
    elseif (ANDROID_ABI STREQUAL "armeabi-v7a")
        set(RUST_TARGET "armv7-linux-androideabi")
    elseif (ANDROID_ABI STREQUAL "x86_64")
        set(RUST_TARGET "x86_64-linux-android")
    endif()
elseif (BEAM_TARGET STREQUAL "iOS")
    if (CMAKE_OSX_SYSROOT MATCHES "simulator")
        set(RUST_TARGET "aarch64-apple-ios-sim")
    else()
        set(RUST_TARGET "aarch64-apple-ios")
    endif()
elseif (BEAM_TARGET STREQUAL "WASM")
    set(RUST_TARGET "wasm32-unknown-emscripten")
else()
    execute_process(COMMAND rustc --print host-triple
                    OUTPUT_VARIABLE RUST_TARGET OUTPUT_STRIP_TRAILING_WHITESPACE)
endif()

set(RUST_LIB_PATH "${RUST_CORE_DIR}/target/${RUST_TARGET}/release/lib${RUST_LIB_NAME}.a")

if (BEAM_TARGET STREQUAL "WASM")
    set(RUST_BUILD_ENV ${CMAKE_COMMAND} -E env "CFLAGS=-fPIC" "RUSTFLAGS=-C relocation-model=pic")
else()
    set(RUST_BUILD_ENV "")
endif()

add_custom_command(
    OUTPUT  "${RUST_LIB_PATH}"
    COMMAND ${RUST_BUILD_ENV} cargo build --release --target "${RUST_TARGET}"
    WORKING_DIRECTORY "${RUST_CORE_DIR}"
    COMMENT "Building Rust core for ${RUST_TARGET}"
)
add_custom_target(beam_rust_core DEPENDS "${RUST_LIB_PATH}")
add_library(BeamRustCore STATIC IMPORTED GLOBAL)
set_target_properties(BeamRustCore PROPERTIES
    IMPORTED_LOCATION "${RUST_LIB_PATH}"
)
add_dependencies(BeamRustCore beam_rust_core)

# ─── C++ ML bridge ───────────────────────────────────────────────────────────

if (BEAM_TARGET STREQUAL "Android")
    add_library(BeamSDK SHARED
        "${REPO_ROOT}/platform/android/tflite_bridge.cpp"
    )
    find_library(log-lib log)
    find_library(android-lib android)
    find_package(TensorFlowLite REQUIRED)

    target_link_libraries(BeamSDK
        BeamRustCore
        TensorFlowLite::TensorFlowLite
        ${log-lib}
        ${android-lib}
    )
    # Compiler flags for ARM performance
    target_compile_options(BeamSDK PRIVATE
        -O3 -march=armv8-a+simd -ffast-math
        -funroll-loops
    )

elseif (BEAM_TARGET STREQUAL "iOS")
    add_library(BeamSDK STATIC
        "${REPO_ROOT}/platform/ios/coreml_bridge.mm"
    )
    find_library(CoreML_LIB CoreML)
    find_library(Foundation_LIB Foundation)
    target_link_libraries(BeamSDK
        BeamRustCore
        ${CoreML_LIB}
        ${Foundation_LIB}
    )
    # XCFramework packaging
    set_target_properties(BeamSDK PROPERTIES
        XCODE_ATTRIBUTE_MACH_O_TYPE staticlib
        FRAMEWORK TRUE
    )

elseif (BEAM_TARGET STREQUAL "WASM")
    add_executable(BeamSDK
        "${REPO_ROOT}/platform/wasm/onnx_bridge.cpp"
    )
    # Emscripten flags
    set_target_properties(BeamSDK PROPERTIES
        LINK_FLAGS
        "-s WASM=1 \
         -s ALLOW_MEMORY_GROWTH=1 \
         -s EXPORTED_RUNTIME_METHODS=['ccall','cwrap'] \
         -s MODULARIZE=1 \
         -s EXPORT_NAME='BeamModule' \
         -s USE_WEBGPU=1"
    )
    target_link_libraries(BeamSDK BeamRustCore)
endif()

# ─── Tests ───────────────────────────────────────────────────────────────────
if (BEAM_TARGET STREQUAL "Host")
    enable_testing()
    # add_subdirectory(tests) # Manual compilation is preferred per ffi_integration_tests.cpp notes
endif()
