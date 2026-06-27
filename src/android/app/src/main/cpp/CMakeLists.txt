cmake_minimum_required(VERSION 3.18.1)

project("lxmusiccore" VERSION 1.0.0 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Export compile commands for IDE support
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# ============================================================================
# Source Files
# ============================================================================

# C++ TurboModule source
set(TURBO_MODULE_SOURCES
    ${CMAKE_CURRENT_SOURCE_DIR}/../cpp/LXMusicModule.cpp
)

# JNI wrapper source (generated)
set(JNI_WRAPPER_SOURCES
    ${CMAKE_CURRENT_SOURCE_DIR}/jni/lx_music_jni.cpp
)

# ============================================================================
# Build Flags
# ============================================================================

# Android NDK specific flags
if(ANDROID)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-rtti -fno-exceptions")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wno-unused-parameter")
    
    # Linker flags
    set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -Wl,--gc-sections")
    set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -llog")
endif()

# ============================================================================
# Library Target
# ============================================================================

add_library(lx_music_core SHARED
    ${TURBO_MODULE_SOURCES}
)

# Include directories
target_include_directories(lx_music_core PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../cpp/include
    ${ANDROID_NDK}/sources/android/native_app_glue
)

# System libraries
target_link_libraries(lx_music_core
    android
    log
)

# Find JNI
find_package(JNI REQUIRED)
if(JNI_FOUND)
    target_include_directories(lx_music_core PRIVATE ${JNI_INCLUDE_DIRS})
    target_link_libraries(lx_music_core ${JNI_LIBRARIES})
endif()

# ============================================================================
# Rust Library (pre-built)
# ============================================================================

# Link against pre-built Rust library
set(RUST_LIB_PATH ${CMAKE_CURRENT_SOURCE_DIR}/../../../src/rust/target/${ANDROID_ABI}/release/liblx_music_core.so)

if(EXISTS ${RUST_LIB_PATH})
    add_library(lx_music_core_rust STATIC IMPORTED)
    set_target_properties(lx_music_core_rust PROPERTIES IMPORTED_LOCATION ${RUST_LIB_PATH})
    target_link_libraries(lx_music_core lx_music_core_rust)
else()
    message(WARNING "Rust library not found at: ${RUST_LIB_PATH}")
    message(STATUS "Please build Rust library first using: cargo ndk -l android")
endif()

# ============================================================================
# Installation
# ============================================================================

# Install to standard location
install(TARGETS lx_music_core
    LIBRARY DESTINATION ${CMAKE_INSTALL_PREFIX}/lib
    ARCHIVE DESTINATION ${CMAKE_INSTALL_PREFIX}/lib
)

# Copy to jniLibs for Android build
install(TARGETS lx_music_core
    LIBRARY DESTINATION ${CMAKE_CURRENT_SOURCE_DIR}/jniLibs/${ANDROID_ABI}
)