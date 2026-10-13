# SPDX-License-Identifier: GPL-3.0-or-later
cmake_minimum_required(VERSION 3.25)
project(forever_casc_prototype LANGUAGES C CXX)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}")
find_package(OpenSSL 3.0 REQUIRED COMPONENTS Crypto)
find_package(Threads REQUIRED)
# Rust resolves -lcrypto through the same library selected by CMake, including
# a development-header-only bootstrap with the installed versioned .so.
file(CREATE_LINK "${OPENSSL_CRYPTO_LIBRARY}" "${CMAKE_BINARY_DIR}/libcrypto.so" SYMBOLIC RESULT link_status)
if(NOT link_status STREQUAL "0")
  message(FATAL_ERROR "Cannot link the selected OpenSSL development library")
endif()
set(pin 02245dcd245e7433e524577656177723d3e4992e)
execute_process(COMMAND git --no-replace-objects -C "${RCE_REFERENCE}" rev-parse HEAD
  OUTPUT_VARIABLE actual_sha OUTPUT_STRIP_TRAILING_WHITESPACE RESULT_VARIABLE status)
if(NOT status EQUAL 0 OR NOT actual_sha STREQUAL pin)
  message(FATAL_ERROR "External CascLib reference is not the pinned revision")
endif()
# Fixed selectors at immutable pin. Never execute upstream CMake, compile its
# hashes/Overwatch/socket implementation, or update the legacy vendor tree.
execute_process(COMMAND git --no-replace-objects -C "${RCE_REFERENCE}" ls-tree
  -r --name-only "${pin}" -- dep/CascLib
  OUTPUT_VARIABLE paths OUTPUT_STRIP_TRAILING_WHITESPACE RESULT_VARIABLE status)
if(NOT status EQUAL 0)
  message(FATAL_ERROR "Cannot enumerate pinned source closure")
endif()
string(REPLACE "\n" ";" paths "${paths}")
set(core_sources)
set(source_root "${CMAKE_BINARY_DIR}/source/dep/CascLib/src")
foreach(path IN LISTS paths)
  if(path STREQUAL "dep/CascLib/LICENSE" OR
     path MATCHES "^dep/CascLib/src/(Casc[^/]*\\.(cpp|h)|common/[^/]*\\.(cpp|h)|jenkins/lookup(3\\.c|\\.h))$")
    if(path STREQUAL "dep/CascLib/src/common/Sockets.cpp")
      continue()
    endif()
    if(IS_SYMLINK "${RCE_REFERENCE}" OR IS_SYMLINK "${RCE_REFERENCE}/dep" OR
       IS_SYMLINK "${RCE_REFERENCE}/dep/CascLib" OR
       IS_SYMLINK "${RCE_REFERENCE}/dep/CascLib/src" OR
       IS_SYMLINK "${RCE_REFERENCE}/${path}")
      message(FATAL_ERROR "Source symlink rejected")
    endif()
    get_filename_component(parent "${RCE_REFERENCE}/${path}" DIRECTORY)
    if(IS_SYMLINK "${parent}")
      message(FATAL_ERROR "Source directory symlink rejected")
    endif()
    execute_process(COMMAND git --no-replace-objects -C "${RCE_REFERENCE}" rev-parse
      "${pin}:${path}" OUTPUT_VARIABLE expected OUTPUT_STRIP_TRAILING_WHITESPACE RESULT_VARIABLE expected_status)
    execute_process(COMMAND git -C "${RCE_REFERENCE}" hash-object "${RCE_REFERENCE}/${path}"
      OUTPUT_VARIABLE actual OUTPUT_STRIP_TRAILING_WHITESPACE RESULT_VARIABLE actual_status)
    if(NOT expected_status EQUAL 0 OR NOT actual_status EQUAL 0 OR NOT actual STREQUAL expected)
      message(FATAL_ERROR "Compiler input differs from immutable source: ${path}")
    endif()
    # COPYONLY preserves upstream notices; no modified upstream file is imported.
    configure_file("${RCE_REFERENCE}/${path}" "${CMAKE_BINARY_DIR}/source/${path}" COPYONLY)
    if(path MATCHES "\\.(cpp|c)$")
      list(APPEND core_sources "${CMAKE_BINARY_DIR}/source/${path}")
    endif()
  endif()
endforeach()
if(NOT core_sources)
  message(FATAL_ERROR "Empty CascLib source closure")
endif()
get_filename_component(repo "${CMAKE_CURRENT_SOURCE_DIR}/../.." ABSOLUTE)
file(GLOB zlib_sources CONFIGURE_DEPENDS "${repo}/vendor/trinitycore-legacy/dep/zlib/*.c")
add_library(forever_zlib STATIC ${zlib_sources})
target_include_directories(forever_zlib PUBLIC "${repo}/vendor/trinitycore-legacy/dep/zlib")
target_compile_definitions(forever_zlib PUBLIC ZLIB_CONST)
add_library(forever_casc_core STATIC ${core_sources} hash_adapter.cpp
  "${repo}/src/compat/CascSocketsDisabled.cpp" "${repo}/src/compat/CascOverwatchDisabled.cpp")
target_include_directories(forever_casc_core PUBLIC "${source_root}"
  PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}" ${RCE_OPENSSL_ARCH_INCLUDE})
target_compile_definitions(forever_casc_core PUBLIC
  CASC_USE_SYSTEM_ZLIB CASCLIB_NO_AUTO_LINK_LIBRARY CASCLIB_NODEBUG)
target_link_libraries(forever_casc_core PUBLIC forever_zlib OpenSSL::Crypto Threads::Threads)
add_library(forever_casc_bridge STATIC bridge.cpp)
target_link_libraries(forever_casc_bridge PUBLIC forever_casc_core)
