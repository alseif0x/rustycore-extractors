// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: supplied synthetic MAPS -> original TerrainBuilder QA.
// Original source/headers/notices remain unmodified in the external pinned tree.
// Access is widened only for the private region entry point in this QA TU.
// No native malformed inputs or missing-file/ancestor paths are exercised.
#include "MMapDefines.h"
#include "WorldModel.h"
#include <G3D/Vector3.h>
#include <boost/filesystem/path.hpp>
#define private public
#include "TerrainBuilder.h"
#undef private
#include <array>
#include <bit>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

void word(std::FILE* f, unsigned value) {
    std::array<unsigned char, 4> b{static_cast<unsigned char>(value), static_cast<unsigned char>(value >> 8), static_cast<unsigned char>(value >> 16), static_cast<unsigned char>(value >> 24)};
    if (std::fwrite(b.data(), 1, 4, f) != 4) throw std::runtime_error("qa_write");
}
template<class T> void values(std::FILE* f, std::vector<T> const& v) {
    word(f, v.size());
    for (T value : v) {
        if constexpr (std::is_same_v<T, float>) word(f, std::bit_cast<unsigned>(value));
        else if constexpr (sizeof(T) == 1) { if (std::fwrite(&value, 1, 1, f) != 1) throw std::runtime_error("qa_write"); }
        else word(f, static_cast<unsigned>(value));
    }
}
int main(int argc, char** argv) try {
    static_assert(sizeof(float) == 4 && sizeof(int) == 4);
    if (argc != 4) throw std::runtime_error("qa_usage");
    auto root = std::filesystem::canonical(argv[1]);
    std::ifstream plan(argv[2]);
    unsigned count, compact, skip, neighborhood;
    if (!(plan >> count >> compact >> skip >> neighborhood) || count < 1 || count > 5 || compact > 1 || skip > 1 || neighborhood > 1) throw std::runtime_error("qa_plan");
    struct Region { unsigned row, col, portion; };
    std::vector<Region> regions;
    for (unsigned i = 0; i < count; ++i) {
        Region r{};
        if (!(plan >> r.row >> r.col >> r.portion) || r.row >= 64 || r.col >= 64 || r.portion < 1 || r.portion > 5) throw std::runtime_error("qa_region");
        char name[80]; std::snprintf(name, sizeof(name), "maps/0000_%02u_%02u.map", r.row, r.col);
        auto p = root / name;
        if (!std::filesystem::is_regular_file(p) || std::filesystem::is_symlink(p) || std::filesystem::file_size(p) > 202112) throw std::runtime_error("qa_input");
        regions.push_back(r);
    }
    std::string tail; if (plan >> tail) throw std::runtime_error("qa_tail");
    MMAP::TerrainBuilder builder(boost::filesystem::path(root.string()), skip != 0);
    MMAP::MeshData data;
    if (neighborhood) {
        auto r = regions[0];
        if (regions.size() != 5 || r.row == 0 || r.row >= 63 || r.col == 0 || r.col >= 63 || r.portion != 5) throw std::runtime_error("qa_neighbors");
        std::array<Region, 4> expected{{{r.row,r.col+1,3},{r.row,r.col-1,2},{r.row+1,r.col,1},{r.row-1,r.col,4}}};
        for (unsigned i=0;i<4;++i)
            if (regions[i+1].row != expected[i].row || regions[i+1].col != expected[i].col || regions[i+1].portion != expected[i].portion) throw std::runtime_error("qa_neighbors");
        // Exercise the original public center/neighbor dispatcher itself.
        builder.loadMap(0, r.row, r.col, data, nullptr);
    } else for (auto r : regions) {
        builder.loadMap(0, r.row, r.col, data, nullptr, static_cast<MMAP::Spot>(r.portion));
    }
    if (compact) {
        MMAP::TerrainBuilder::cleanVertices(data.solidVerts, data.solidTris);
        MMAP::TerrainBuilder::cleanVertices(data.liquidVerts, data.liquidTris);
    }
    std::FILE* output = std::fopen(argv[3], "wbx");
    if (!output) throw std::runtime_error("qa_output_exists");
    values(output, data.solidVerts); values(output, data.solidTris);
    values(output, data.liquidVerts); values(output, data.liquidTris); values(output, data.liquidType);
    if (std::fclose(output)) throw std::runtime_error("qa_close");
    std::cout << "{\"ok\":true}\n";
    return 0;
} catch (std::exception const&) { std::cerr << "{\"error\":\"native_navigation_qa_failed\"}\n"; return 4; }
