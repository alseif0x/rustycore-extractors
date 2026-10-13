// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only original ADTFile::initFromCache caller.
// Original adtfile.cpp and headers remain unmodified in an external closure.
// A standard explicit-template-instantiation access helper sets the private
// cache pointer without changing class declarations, layout or source code.
// Cache input is complete dir_bin: original writers' cached Flags clear bit2
// and Data retains every subsequent byte, separately verified by native direct
// and doodad QA. This adapter tests the original REPLAY function, not discovery,
// cache construction by a full ADT parser, IDs or acquisition.
//
// RCECCH01: magic8, mapID/originalMapID/count/byteCount as four u32 LE,
// then byteCount complete dir_bin bytes. Maximum 1MiB/10000 records, names
// ASCII nonempty/noNUL <=500 and finite ordered records. CLI: fixture new-stage.
// Output stays in original dir_bin/mapName inside a new0700 stage. Empty cache
// must succeed without creating a file. Stdout only counts/status. Synthetic
// ordinary input files are bounded, not hardened against concurrent mutation.
#include "adtfile.h"
#include "vmapexport.h"
#include <bit>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <string_view>
#include <sys/stat.h>

namespace {
using Bytes = std::vector<uint8>;
struct CacheMember {
    using type = std::vector<ADTOutputCache>* ADTFile::*;
    friend type cache_member(CacheMember);
};
template<CacheMember::type Member> struct CacheAccess {
    friend CacheMember::type cache_member(CacheMember) { return Member; }
};
// Access control is not applied to names in explicit instantiations.
template struct CacheAccess<&ADTFile::dirfileCache>;
unsigned constructors = 0;
unsigned closes = 0;
uint32 word(Bytes const& b, std::size_t at) {
    return uint32(b.at(at)) | (uint32(b.at(at + 1)) << 8)
        | (uint32(b.at(at + 2)) << 16) | (uint32(b.at(at + 3)) << 24);
}
float real(Bytes const& b, std::size_t at) { return std::bit_cast<float>(word(b, at)); }
}

char const* szWorkDirWmo = nullptr;
std::shared_ptr<CASC::Storage> CascStorage;
// Narrow lifetime adapter: a single deliberately empty synthetic file. No
// permissive read/open implementation or real CASC backend is linked.
CASCFile::CASCFile(std::shared_ptr<CASC::Storage const> storage, char const* name, bool)
    : eof(true), buffer(nullptr), pointer(0), size(0) {
    if (storage || std::string_view(name) != "synthetic-cache" || constructors++)
        std::abort();
}
void CASCFile::close() {
    if (buffer || pointer || size || !eof) std::abort();
    ++closes;
}

int main(int argc, char** argv) try {
    static_assert(std::endian::native == std::endian::little);
    if (argc != 3) throw "qa_usage";
    ::umask(0077);
    std::ifstream input(argv[1], std::ios::binary | std::ios::ate);
    auto length = input.tellg();
    if (length < 24 || length > 24 + 1024 * 1024) throw "qa_extent";
    input.seekg(0);
    Bytes bytes(static_cast<std::size_t>(length));
    if (!input.read(reinterpret_cast<char*>(bytes.data()), length)) throw "qa_input";
    if (std::string_view(reinterpret_cast<char const*>(bytes.data()), 8) != "RCECCH01")
        throw "qa_magic";
    uint32 map = word(bytes, 8), original = word(bytes, 12);
    uint32 count = word(bytes, 16), size = word(bytes, 20);
    if (count > 10000 || size != bytes.size() - 24) throw "qa_extent";
    auto cache = std::make_unique<std::vector<ADTOutputCache>>();
    cache->reserve(count);
    std::size_t at = 24;
    for (uint32 i = 0; i < count; ++i) {
        uint8 flags = bytes.at(at);
        if (flags & ~7u) throw "qa_flags";
        auto name_at = at + ((flags & 1) ? 58u : 34u);
        auto name_size = word(bytes, name_at);
        auto end = name_at + 4 + name_size;
        if (!name_size || name_size > 500 || end > bytes.size()) throw "qa_name";
        for (auto j = name_at + 4; j < end; ++j)
            if (!bytes[j] || bytes[j] > 127) throw "qa_name";
        for (auto j = at + 6; j < at + ((flags & 1) ? 58u : 34u); j += 4)
            if (!std::isfinite(real(bytes, j))) throw "qa_nonfinite";
        if (flags & 1)
            for (unsigned axis = 0; axis < 3; ++axis)
                if (real(bytes, at + 34 + axis * 4) > real(bytes, at + 46 + axis * 4))
                    throw "qa_bounds";
        cache->push_back({uint8(flags & ~2u), Bytes(bytes.begin() + at + 1, bytes.begin() + end)});
        at = end;
    }
    if (at != bytes.size()) throw "qa_tail";
    auto stage = std::filesystem::absolute(argv[2]);
    if (!std::filesystem::create_directory(stage)) throw "qa_stage_exists";
    if (!std::filesystem::create_directory(stage / "dir_bin")) throw "qa_stage";
    auto root = stage.string();
    szWorkDirWmo = root.c_str();
    bool result = false;
    {
        ADTFile adt("synthetic-cache", true);
        adt.*cache_member(CacheMember{}) = cache.release();
        result = adt.initFromCache(map, original);
    }
    if (!result || constructors != 1 || closes != 2 || CascStorage) throw "qa_replay";
    std::ostringstream leaf;
    leaf << std::setfill('0') << std::setw(4) << map;
    auto out = stage / "dir_bin" / leaf.str();
    if (count == 0 ? std::filesystem::exists(out)
                   : (!std::filesystem::is_regular_file(out) || std::filesystem::file_size(out) != size))
        throw "qa_output_extent";
    std::printf("{\"status\":\"ok\",\"records\":%u,\"bytes\":%u,\"replay_calls\":1,\"constructor_calls\":1,\"close_calls\":2,\"empty_file_absent\":%s}\n",
        count, size, count ? "false" : "true");
    return 0;
} catch (char const* code) {
    std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n", code);
    return 4;
} catch (...) {
    std::puts("{\"status\":\"error\",\"code\":\"qa_failure\"}");
    return 4;
}
