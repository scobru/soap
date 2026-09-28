# Wraps the CLAP built by `cargo xtask bundle` into VST3 (all platforms) and
# AUv2 (macOS) with clap-wrapper. The wrappers carry no DSP of their own: they
# host the CLAP, which macOS embeds in each bundle (Contents/PlugIns) and
# Windows/Linux load from the CLAP folder by name.
cmake_minimum_required(VERSION 3.21)

if(APPLE)
    set(CMAKE_OSX_ARCHITECTURES "arm64" CACHE STRING "Clear ships Apple Silicon only")
    set(CMAKE_OSX_DEPLOYMENT_TARGET "14.0" CACHE STRING "Clear needs macOS 14")
endif()

project(remove_that_dirt_wrappers VERSION 0.1.0 LANGUAGES C CXX)
if(APPLE)
    enable_language(OBJC OBJCXX)
endif()

set(CMAKE_POSITION_INDEPENDENT_CODE ON)
set(RTD_NAME "Remove That Dirt")
set(RTD_CLAP_BUNDLE "" CACHE PATH "Path to the built '${RTD_NAME}.clap' (embedded on macOS)")

set(CLAP_WRAPPER_DOWNLOAD_DEPENDENCIES TRUE CACHE BOOL "Fetch the CLAP, VST3 and AudioUnit SDKs" FORCE)
include(FetchContent)
FetchContent_Declare(
    clap-wrapper
    GIT_REPOSITORY https://github.com/free-audio/clap-wrapper.git
    GIT_TAG 1cca996e96f29ab2be7ae9f8cfe532bbc92e1dd6
)
FetchContent_MakeAvailable(clap-wrapper)

if(APPLE AND RTD_CLAP_BUNDLE STREQUAL "")
    message(FATAL_ERROR "Set -DRTD_CLAP_BUNDLE=/path/to/${RTD_NAME}.clap")
endif()

add_library(rtd_vst3 MODULE)
target_add_vst3_wrapper(
    TARGET rtd_vst3
    OUTPUT_NAME "${RTD_NAME}"
    BUNDLE_IDENTIFIER "com.scobru.remove-that-dirt.vst3"
    BUNDLE_VERSION "${PROJECT_VERSION}"
    MACOS_EMBEDDED_CLAP_LOCATION "${RTD_CLAP_BUNDLE}"
)

if(APPLE)
    add_library(rtd_auv2 MODULE)
    target_add_auv2_wrapper(
        TARGET rtd_auv2
        OUTPUT_NAME "${RTD_NAME}"
        BUNDLE_IDENTIFIER "com.scobru.remove-that-dirt.component"
        BUNDLE_VERSION "${PROJECT_VERSION}"
        INSTRUMENT_TYPE "aufx"
        MANUFACTURER_NAME "scobru"
        MANUFACTURER_CODE "Scbr"
        SUBTYPE_CODE "Rtdt"
        MACOS_EMBEDDED_CLAP_LOCATION "${RTD_CLAP_BUNDLE}"
    )
endif()
