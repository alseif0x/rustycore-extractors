// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "StandaloneMetadata.h"
#include "StandaloneBuildInfo.h"
#include <cstdio>
#include <cstring>

bool RustyCore::Extractors::HandleMetadataRequest(int argc, char** argv, char const* toolName)
{
    if (argc != 2)
        return false;

    if (std::strcmp(argv[1], "--version") == 0)
    {
        std::printf("%s %s\n", toolName, RCE_VERSION);
        std::printf("rustycore_commit=%s\n", RCE_GIT_COMMIT);
        std::printf("upstream_commit=%s\n", RCE_UPSTREAM_COMMIT);
        std::printf("client=%s %s.%d\n", RCE_SUPPORTED_PRODUCT, RCE_SUPPORTED_VERSION, RCE_SUPPORTED_BUILD);
        std::printf("formats=MAPS/10,VMAP_4.B,MMAP/15,DETOUR/7/64-bit\n");
        return true;
    }

    if (std::strcmp(argv[1], "--version-json") == 0)
    {
        std::printf(
            "{\"tool\":\"%s\",\"version\":\"%s\",\"commit\":\"%s\","
            "\"upstream_commit\":\"%s\",\"client\":{\"product\":\"%s\","
            "\"version\":\"%s\",\"build\":%d},\"formats\":{\"map\":{\"magic\":"
            "\"MAPS\",\"version\":10},\"vmap\":{\"magic\":\"VMAP_4.B\","
            "\"raw_magic\":\"VMAP04B\"},\"mmap\":{\"magic\":\"MMAP\","
            "\"version\":15,\"detour_navmesh_version\":7,\"detour_polyref_bits\":64}}}\n",
            toolName, RCE_VERSION, RCE_GIT_COMMIT, RCE_UPSTREAM_COMMIT,
            RCE_SUPPORTED_PRODUCT, RCE_SUPPORTED_VERSION, RCE_SUPPORTED_BUILD);
        return true;
    }

    return false;
}
