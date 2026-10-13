// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only serial caller of the original pinned
// GenerateUniqueObjectId. Compile unmodified vmapexport.cpp externally with
// its main renamed and function/data sections plus linker GC. This adapter
// performs no extraction, acquisition, threading or ID arithmetic.
// Input: RCEID001, u32 LE count <=100000, repeated {client:u32,doodad:u16,
// isWmo:u8 (0/1),reserved:u8(0)}. Output same magic/count and resulting u32s.
// A fresh process resets original static state. Numeric collisions between
// distinct keys remain observable; the Rust policy deliberately rejects them.
// vmapexport.cpp has a process-global empty shared CASC handle. The linker
// retains its deleter; a fail-fast destructor below must never execute. No
// acquisition implementation is substituted or allowed to succeed.
#include "vmapexport.h"
#include "CascHandles.h"
#include <bit>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <string_view>
#include <vector>

extern std::shared_ptr<CASC::Storage> CascStorage;
CASC::Storage::~Storage() { std::abort(); }

int main(int argc, char** argv) try {
    static_assert(std::endian::native == std::endian::little);
    if (CascStorage) throw "storage_guard";
    if (argc != 2) throw "arguments";
    std::ifstream input(argv[1], std::ios::binary | std::ios::ate);
    auto size = input.tellg();
    if (size < 12 || size > 800012) throw "extent";
    input.seekg(0);
    std::vector<unsigned char> bytes(static_cast<std::size_t>(size));
    if (!input.read(reinterpret_cast<char*>(bytes.data()), size)) throw "read";
    auto word = [&](std::size_t at) {
        return uint32(bytes.at(at)) | (uint32(bytes.at(at + 1)) << 8)
            | (uint32(bytes.at(at + 2)) << 16) | (uint32(bytes.at(at + 3)) << 24);
    };
    if (std::string_view(reinterpret_cast<char const*>(bytes.data()), 8) != "RCEID001") throw "magic";
    auto count = word(8);
    if (count > 100000 || bytes.size() != 12 + std::size_t(count) * 8) throw "count";
    // Validate the whole synthetic request before mutating original state.
    for (std::size_t at = 12; at < bytes.size(); at += 8)
        if (bytes[at + 6] > 1 || bytes[at + 7] != 0) throw "record";
    std::vector<uint32> result;
    result.reserve(count);
    for (std::size_t at = 12; at < bytes.size(); at += 8) {
        auto doodad = uint16(bytes[at + 4]) | (uint16(bytes[at + 5]) << 8);
        result.push_back(GenerateUniqueObjectId(word(at), doodad, bytes[at + 6] != 0));
    }
    if (CascStorage) throw "storage_guard";
    std::cout.write("RCEID001", 8);
    std::cout.write(reinterpret_cast<char const*>(&count), 4);
    for (auto value : result)
        std::cout.write(reinterpret_cast<char const*>(&value), 4);
    return std::cout ? 0 : 4;
} catch (...) { return 4; }
