# Template CMake Tercepat untuk C++ / Android NDK:
# - ccache (compiler cache)
# - Ninja generator
# - mold / lld linker
# - Unity Build (batch cpp)
# - Precompiled Headers (PCH)

cmake_minimum_required(VERSION 3.16)
project(FastCppEngine LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 1. Aktifkan ccache otomatis jika terinstall
find_program(CCACHE_PROGRAM ccache)
if(CCACHE_PROGRAM)
    message(STATUS "⚡ ccache ditemukan: ${CCACHE_PROGRAM}")
    set(CMAKE_C_COMPILER_LAUNCHER "${CCACHE_PROGRAM}")
    set(CMAKE_CXX_COMPILER_LAUNCHER "${CCACHE_PROGRAM}")
endif()

# 2. Gunakan Linker Mold atau LLD (menghindari bottleneck GNU ld)
find_program(MOLD_PROGRAM mold)
if(MOLD_PROGRAM)
    message(STATUS "⚡ Menggunakan mold linker super-cepat")
    add_link_options("-fuse-ld=mold")
else()
    message(STATUS "⚡ Mold tidak ditemukan, fallback ke lld")
    add_link_options("-fuse-ld=lld")
endif()

# 3. Source Library Contoh
add_library(fast_engine SHARED
    src/engine.cpp
    src/network.cpp
)

# 4. Aktifkan Unity Build (Memangkas include overhead hingga 60%)
set_target_properties(fast_engine PROPERTIES
    UNITY_BUILD ON
    UNITY_BUILD_BATCH_SIZE 16
)

# 5. Precompiled Headers (PCH) untuk header berat
target_precompile_headers(fast_engine PRIVATE
    <vector>
    <string>
    <memory>
    <unordered_map>
    <thread>
    <future>
)
