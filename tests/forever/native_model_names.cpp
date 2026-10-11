// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only caller of the original pinned
// GetPlainName and NormalizeFileName in adtfile.cpp. Original files and
// notices are materialized externally, unmodified. No acquisition, model
// conversion, full exporter or filename-based filesystem access executes.
// Input: RCEFN001, LE u32 count, repeated LE u32 length + ASCII source name.
// Output: the same framing with each original normalized basename.
#include "adtfile.h"
#include <bit>
#include <fstream>
#include <iostream>
#include <span>
#include <string>
#include <vector>

int main(int argc, char** argv) try {
    static_assert(std::endian::native == std::endian::little);
    if (argc != 2) throw "arguments";
    std::ifstream input(argv[1], std::ios::binary | std::ios::ate);
    auto size = input.tellg();
    if (size < 12 || size > 1024 * 1024) throw "extent";
    input.seekg(0);
    std::vector<char> bytes(static_cast<std::size_t>(size));
    if (!input.read(bytes.data(), size)) throw "read";
    std::size_t at = 8;
    auto word = [&]() {
        if (at > bytes.size() || bytes.size() - at < 4) throw "extent";
        uint32 result = 0;
        for (unsigned i = 0; i < 4; ++i)
            result |= uint32(static_cast<unsigned char>(bytes[at++])) << (8 * i);
        return result;
    };
    if (std::string_view(bytes.data(), 8) != "RCEFN001") throw "magic";
    auto count = word();
    if (count > 10000) throw "count";
    std::vector<std::string> results;
    results.reserve(count);
    for (uint32 i = 0; i < count; ++i) {
        auto length = word();
        if (!length || length > 1024 || length > bytes.size() - at) throw "length";
        std::string_view source(bytes.data() + at, length);
        for (unsigned char c : source)
            if (c < 32 || c > 126) throw "ascii";
        at += length;
        std::string name(GetPlainName(source));
        if (name.empty()) throw "empty_basename";
        NormalizeFileName(name);
        results.push_back(std::move(name));
    }
    if (at != bytes.size()) throw "tail";
    std::cout.write("RCEFN001", 8);
    std::cout.write(reinterpret_cast<char const*>(&count), 4);
    for (auto const& name : results) {
        uint32 length = name.size();
        std::cout.write(reinterpret_cast<char const*>(&length), 4);
        std::cout.write(name.data(), length);
    }
    return std::cout ? 0 : 4;
} catch (...) { return 4; }
