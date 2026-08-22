// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#define __CASCLIB_SELF__
#include "CascLib.h"
#include "CascCommon.h"
#include "CascHandles.h"
#include "G3D/g3dfnmatch.h"
#include "hashes/md5.h"
#include "hashes/sha1.h"
#include "common/Mime.h"
#include "common/Sockets.h"
#include <boost/filesystem/path.hpp>
#include <array>
#include <cstdio>
#include <cstring>

namespace
{
template <std::size_t N>
bool Equal(std::array<unsigned char, N> const& actual, std::array<unsigned char, N> const& expected, char const* label)
{
    if (actual == expected)
        return true;
    std::fprintf(stderr, "%s digest mismatch\n", label);
    return false;
}
}

int main()
{
    char const input[] = "abc";

    MD5_CTX md5{};
    std::array<unsigned char, 16> md5Result{};
    MD5_Init(&md5);
    MD5_Update(&md5, input, 3);
    MD5_Final(md5Result.data(), &md5);
    std::array<unsigned char, 16> const expectedMd5 = {
        0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0,
        0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1, 0x7f, 0x72};

    SHA1_CTX sha1{};
    std::array<unsigned char, 20> sha1Result{};
    SHA1_Init(&sha1);
    SHA1_Update(&sha1, input, 3);
    SHA1_Final(&sha1, sha1Result.data());
    std::array<unsigned char, 20> const expectedSha1 = {
        0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e,
        0x25, 0x71, 0x78, 0x50, 0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d};

    bool ok = Equal(md5Result, expectedMd5, "MD5") && Equal(sha1Result, expectedSha1, "SHA1");
    ok = ok && G3D::g3dfnmatch("*.vmtree", "0571.vmtree", 0) == 0;
    ok = ok && G3D::g3dfnmatch("*.vmtree", "0571.vmtile", 0) == FNM_NOMATCH;
    ok = ok && G3D::g3dfnmatch("MAPS/*.MAP", "maps/0001.map", FNM_CASEFOLD | FNM_PATHNAME) == 0;
    ok = ok && G3D::g3dfnmatch("*", ".hidden", FNM_PERIOD) == FNM_NOMATCH;

    CASC::Storage* remote = CASC::Storage::OpenRemote(boost::filesystem::path("unused"), 0, "wow_classic", "eu");
    ok = ok && remote == nullptr;
    delete remote;
    ok = ok && sockets_connect("invalid.example", CASC_PORT_HTTP) == nullptr;

    return ok ? 0 : 1;
}
