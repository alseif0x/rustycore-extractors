# Copyright (C) 2026 rustycore-extractors contributors
# SPDX-License-Identifier: GPL-3.0-or-later

cmake_minimum_required(VERSION 3.25)

project(rustycore-extractors VERSION 0.1.0 LANGUAGES C CXX)

include(CTest)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

if(WIN32)
  add_compile_definitions(
    _WIN32_WINNT=0x0A00
    NTDDI_VERSION=0x0A000007
    WIN32_LEAN_AND_MEAN
    NOMINMAX
    TRINITY_REQUIRED_WINDOWS_BUILD=18362)
endif()

include(GNUInstallDirs)
find_package(OpenSSL 3.0 REQUIRED COMPONENTS Crypto)
find_package(Threads REQUIRED)

set(RCE_UPSTREAM_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/vendor/trinitycore-legacy")
set(RCE_DEP_ROOT "${RCE_UPSTREAM_ROOT}/dep")
set(RCE_COMMON_ROOT "${RCE_UPSTREAM_ROOT}/src/common")
set(RCE_TOOLS_ROOT "${RCE_UPSTREAM_ROOT}/src/tools")

execute_process(
  COMMAND git rev-parse HEAD
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  OUTPUT_VARIABLE RCE_GIT_COMMIT
  OUTPUT_STRIP_TRAILING_WHITESPACE
  ERROR_QUIET)
string(LENGTH "${RCE_GIT_COMMIT}" RCE_GIT_COMMIT_LENGTH)
if(NOT RCE_GIT_COMMIT_LENGTH EQUAL 40 OR NOT RCE_GIT_COMMIT MATCHES "^[0-9a-f]+$")
  set(RCE_GIT_COMMIT "0000000000000000000000000000000000000000")
endif()

set(RCE_UPSTREAM_COMMIT "92796557f9b0ba3d2d1c7c770f535153154cf83e")
set(RCE_SUPPORTED_PRODUCT "wow_classic")
set(RCE_SUPPORTED_VERSION "3.4.3")
set(RCE_SUPPORTED_BUILD 51943)

configure_file(cmake/revision_data.h.in generated/revision_data.h @ONLY)
configure_file(cmake/StandaloneBuildInfo.h.in generated/StandaloneBuildInfo.h @ONLY)

set(RCE_COMMON_INCLUDE_DIRS
  "${CMAKE_CURRENT_BINARY_DIR}/generated"
  "${CMAKE_CURRENT_SOURCE_DIR}/src/compat"
  "${RCE_COMMON_ROOT}"
  "${RCE_COMMON_ROOT}/Asio"
  "${RCE_COMMON_ROOT}/Collision"
  "${RCE_COMMON_ROOT}/Collision/Management"
  "${RCE_COMMON_ROOT}/Collision/Maps"
  "${RCE_COMMON_ROOT}/Collision/Models"
  "${RCE_COMMON_ROOT}/Containers/Utilities"
  "${RCE_COMMON_ROOT}/DataStores"
  "${RCE_COMMON_ROOT}/Debugging"
  "${RCE_COMMON_ROOT}/Threading"
  "${RCE_COMMON_ROOT}/Time"
  "${RCE_COMMON_ROOT}/Utilities")

file(GLOB RCE_ZLIB_SOURCES CONFIGURE_DEPENDS "${RCE_DEP_ROOT}/zlib/*.c")
add_library(rce_zlib STATIC ${RCE_ZLIB_SOURCES})
target_include_directories(rce_zlib PUBLIC "${RCE_DEP_ROOT}/zlib")
target_compile_definitions(rce_zlib PUBLIC ZLIB_CONST)

file(GLOB RCE_FMT_SOURCES CONFIGURE_DEPENDS "${RCE_DEP_ROOT}/fmt/src/*.cc")
add_library(rce_fmt STATIC ${RCE_FMT_SOURCES})
target_include_directories(rce_fmt PUBLIC "${RCE_DEP_ROOT}/fmt/include")

file(GLOB RCE_DETOUR_SOURCES CONFIGURE_DEPENDS "${RCE_DEP_ROOT}/recastnavigation/Detour/Source/*.cpp")
add_library(rce_detour STATIC ${RCE_DETOUR_SOURCES})
target_include_directories(rce_detour PUBLIC "${RCE_DEP_ROOT}/recastnavigation/Detour/Include")

file(GLOB RCE_RECAST_SOURCES CONFIGURE_DEPENDS "${RCE_DEP_ROOT}/recastnavigation/Recast/Source/*.cpp")
add_library(rce_recast STATIC ${RCE_RECAST_SOURCES})
target_include_directories(rce_recast PUBLIC "${RCE_DEP_ROOT}/recastnavigation/Recast/Include")

file(GLOB RCE_G3D_SOURCES CONFIGURE_DEPENDS "${RCE_DEP_ROOT}/g3dlite/source/*.cpp")
add_library(rce_g3d STATIC ${RCE_G3D_SOURCES} src/compat/g3dfnmatch.cpp)
target_include_directories(rce_g3d
  PUBLIC "${RCE_DEP_ROOT}/g3dlite/include"
  PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/compat")
target_compile_definitions(rce_g3d PRIVATE _HAVE_ZIP=0)
target_link_libraries(rce_g3d PUBLIC rce_zlib Threads::Threads)
if(WIN32)
  target_link_libraries(rce_g3d PUBLIC advapi32 psapi winmm)
endif()

file(GLOB RCE_CASC_ROOT_SOURCES CONFIGURE_DEPENDS "${RCE_DEP_ROOT}/CascLib/src/*.cpp")
file(GLOB RCE_CASC_COMMON_SOURCES CONFIGURE_DEPENDS "${RCE_DEP_ROOT}/CascLib/src/common/*.cpp")
list(FILTER RCE_CASC_COMMON_SOURCES EXCLUDE REGEX "/Sockets\\.cpp$")
add_library(rce_casc STATIC
  ${RCE_CASC_ROOT_SOURCES}
  ${RCE_CASC_COMMON_SOURCES}
  "${RCE_DEP_ROOT}/CascLib/src/jenkins/lookup3.c"
  src/compat/CascHashAdapters.cpp
  src/compat/CascOverwatchDisabled.cpp
  src/compat/CascSocketsDisabled.cpp)
target_include_directories(rce_casc
  PUBLIC "${RCE_DEP_ROOT}/CascLib/src"
  PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/compat" "${RCE_DEP_ROOT}")
target_compile_definitions(rce_casc PUBLIC
  __SYS_ZLIB
  CASCLIB_NO_AUTO_LINK_LIBRARY
  CASCLIB_NODEBUG)
target_link_libraries(rce_casc PUBLIC rce_zlib OpenSSL::Crypto)
if(WIN32)
  target_link_libraries(rce_casc PUBLIC ws2_32)
endif()

file(GLOB_RECURSE RCE_COMMON_SOURCES CONFIGURE_DEPENDS "${RCE_COMMON_ROOT}/*.cpp")
add_library(rce_common STATIC
  ${RCE_COMMON_SOURCES}
  src/compat/StandaloneAdapters.cpp
  src/compat/StandaloneMetadata.cpp)
target_include_directories(rce_common PUBLIC ${RCE_COMMON_INCLUDE_DIRS})
target_compile_definitions(rce_common PUBLIC TRINITY_API_EXPORT_COMMON)
target_link_libraries(rce_common
  PUBLIC rce_fmt rce_g3d rce_detour Threads::Threads)

add_library(rce_extractor_common STATIC
  "${RCE_TOOLS_ROOT}/extractor_common/CascHandles.cpp"
  "${RCE_TOOLS_ROOT}/extractor_common/DB2CascFileSource.cpp")
target_include_directories(rce_extractor_common PUBLIC
  ${RCE_COMMON_INCLUDE_DIRS}
  "${RCE_TOOLS_ROOT}/extractor_common"
  "${RCE_DEP_ROOT}/CascLib/src")
target_link_libraries(rce_extractor_common PUBLIC rce_common rce_casc)

add_executable(mapextractor
  "${RCE_TOOLS_ROOT}/map_extractor/System.cpp"
  "${RCE_TOOLS_ROOT}/map_extractor/loadlib.cpp")
target_include_directories(mapextractor PRIVATE
  ${RCE_COMMON_INCLUDE_DIRS}
  "${RCE_TOOLS_ROOT}/map_extractor"
  "${RCE_TOOLS_ROOT}/map_extractor/loadlib"
  "${RCE_TOOLS_ROOT}/extractor_common")
target_link_libraries(mapextractor PRIVATE rce_extractor_common)

file(GLOB RCE_VMAP_EXTRACTOR_SOURCES CONFIGURE_DEPENDS "${RCE_TOOLS_ROOT}/vmap4_extractor/*.cpp")
add_executable(vmap4extractor ${RCE_VMAP_EXTRACTOR_SOURCES})
target_include_directories(vmap4extractor PRIVATE
  ${RCE_COMMON_INCLUDE_DIRS}
  "${RCE_TOOLS_ROOT}/vmap4_extractor"
  "${RCE_TOOLS_ROOT}/extractor_common")
target_link_libraries(vmap4extractor PRIVATE rce_extractor_common)

add_executable(vmap4assembler
  "${RCE_TOOLS_ROOT}/vmap4_assembler/TileAssembler.cpp"
  "${RCE_TOOLS_ROOT}/vmap4_assembler/VMapAssembler.cpp")
target_include_directories(vmap4assembler PRIVATE
  ${RCE_COMMON_INCLUDE_DIRS}
  "${RCE_TOOLS_ROOT}/vmap4_assembler")
target_link_libraries(vmap4assembler PRIVATE rce_common rce_casc)

add_executable(mmaps_generator
  "${RCE_TOOLS_ROOT}/mmaps_generator/IntermediateValues.cpp"
  "${RCE_TOOLS_ROOT}/mmaps_generator/MapBuilder.cpp"
  "${RCE_TOOLS_ROOT}/mmaps_generator/PathGenerator.cpp"
  "${RCE_TOOLS_ROOT}/mmaps_generator/TerrainBuilder.cpp")
target_include_directories(mmaps_generator PRIVATE
  ${RCE_COMMON_INCLUDE_DIRS}
  "${RCE_TOOLS_ROOT}/mmaps_generator"
  "${RCE_TOOLS_ROOT}/extractor_common"
  "${RCE_DEP_ROOT}/recastnavigation/Recast/Include"
  "${RCE_DEP_ROOT}/recastnavigation/Detour/Include")
target_link_libraries(mmaps_generator PRIVATE
  rce_common rce_extractor_common rce_recast rce_detour rce_zlib)

foreach(tool IN ITEMS mapextractor vmap4extractor vmap4assembler mmaps_generator)
  install(TARGETS ${tool} RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}")
endforeach()

if(BUILD_TESTING)
  find_package(Python3 REQUIRED COMPONENTS Interpreter)
  add_test(
    NAME source-closure
    COMMAND "${Python3_EXECUTABLE}"
      "${CMAKE_CURRENT_SOURCE_DIR}/tests/provenance/test_source_closure.py")

  add_test(
    NAME conformance-harness
    COMMAND "${Python3_EXECUTABLE}" -m unittest discover
      -s "${CMAKE_CURRENT_SOURCE_DIR}/tests/conformance"
      -p "test_*.py" -v)

  add_executable(rce_compat_tests tests/cpp/compat_tests.cpp)
  target_include_directories(rce_compat_tests PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/src/compat"
    "${RCE_TOOLS_ROOT}/extractor_common"
    "${RCE_DEP_ROOT}/CascLib/src"
    "${RCE_DEP_ROOT}")
  target_link_libraries(rce_compat_tests PRIVATE rce_extractor_common rce_g3d)
  add_test(NAME compat-adapters COMMAND rce_compat_tests)

  foreach(tool IN ITEMS mapextractor vmap4extractor vmap4assembler mmaps_generator)
    add_test(
      NAME "metadata-${tool}"
      COMMAND "${CMAKE_COMMAND}"
        "-DTOOL=$<TARGET_FILE:${tool}>"
        "-DEXPECTED_NAME=${tool}"
        -P "${CMAKE_CURRENT_SOURCE_DIR}/tests/cmake/ValidateMetadata.cmake")
  endforeach()
endif()
