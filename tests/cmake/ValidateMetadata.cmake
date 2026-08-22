# Copyright (C) 2026 rustycore-extractors contributors
# SPDX-License-Identifier: GPL-3.0-or-later

if(NOT DEFINED TOOL OR NOT DEFINED EXPECTED_NAME)
  message(FATAL_ERROR "TOOL and EXPECTED_NAME are required")
endif()

execute_process(
  COMMAND "${TOOL}" --version-json
  RESULT_VARIABLE status
  OUTPUT_VARIABLE metadata
  ERROR_VARIABLE error)
if(NOT status EQUAL 0)
  message(FATAL_ERROR "${EXPECTED_NAME} metadata failed: ${error}")
endif()

string(JSON name GET "${metadata}" tool)
string(JSON product GET "${metadata}" client product)
string(JSON build GET "${metadata}" client build)
string(JSON map_version GET "${metadata}" formats map version)
string(JSON mmap_version GET "${metadata}" formats mmap version)
string(JSON detour_version GET "${metadata}" formats mmap detour_navmesh_version)

if(NOT name STREQUAL EXPECTED_NAME)
  message(FATAL_ERROR "Unexpected tool name: ${name}")
endif()
if(NOT product STREQUAL "wow_classic" OR NOT build EQUAL 51943)
  message(FATAL_ERROR "Unexpected client target: ${product}/${build}")
endif()
if(NOT map_version EQUAL 10 OR NOT mmap_version EQUAL 15 OR NOT detour_version EQUAL 7)
  message(FATAL_ERROR "Unexpected format metadata")
endif()
