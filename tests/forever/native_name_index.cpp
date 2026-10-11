// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only adapter to the original pinned CascLib
// FileTree and CalcFileNameHash. This is not a ROOT/storage parser or extractor.
#include "CascLib.h"
#include "CascCommon.h"
#include <array>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <iterator>
#include <string>
#include <vector>

namespace {
struct Input {
    std::vector<unsigned char> bytes;
    std::size_t at = 0;
    std::uint64_t number(unsigned size) {
        if (size > 8 || at > bytes.size() || size > bytes.size() - at) throw 1;
        std::uint64_t n = 0;
        for (unsigned i = 0; i != size; ++i) n |= std::uint64_t(bytes[at++]) << (i * 8);
        return n;
    }
    std::string name() {
        auto n = number(4);
        if (!n || n > 1024 || n > bytes.size() - at) throw 1;
        std::string s;
        for (std::size_t i = 0; i != n; ++i) {
            auto c = number(1);
            if (!c || c > 127) throw 1;
            s.push_back(char(c));
        }
        return s;
    }
};
void emit(std::uint64_t n, unsigned size) {
    for (unsigned i = 0; i != size; ++i) if (std::putchar((n >> (8 * i)) & 255) == EOF) throw 2;
}
struct Block { std::uint32_t locale, content, start, end; };
struct Entry { std::uint32_t id, key; std::uint64_t hash; bool present; };
struct Tree {
    CASC_FILE_TREE value{};
    Tree() { if (value.Create(FTREE_FLAG_USE_DATA_ID | FTREE_FLAG_USE_LOCALE_FLAGS | FTREE_FLAG_USE_CONTENT_FLAGS)) throw 3; }
    ~Tree() { value.Free(); }
};
}

int main(int argc, char** argv) try {
    if (argc != 2) return 1;
    std::ifstream file(argv[1], std::ios::binary);
    if (!file) return 1;
    Input input{{std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>()}};
    if (input.bytes.size() > 16 * 1024 * 1024 || input.bytes.size() < 24 ||
        std::memcmp(input.bytes.data(), "RCENAM01", 8)) return 1;
    input.at = 8;
    auto locale = input.number(4), nb = input.number(4), ne = input.number(4), nq = input.number(4);
    if ((locale != 2 && locale != 0x80) || nb > 65536 || ne > 100000 || nq > 100000) return 1;
    std::vector<Block> blocks;
    std::uint32_t cursor = 0;
    for (std::size_t i = 0; i != nb; ++i) {
        auto l = input.number(4), c = input.number(4), n = input.number(4);
        if (n > ne - cursor) return 1;
        blocks.push_back({std::uint32_t(l), std::uint32_t(c), cursor, std::uint32_t(cursor+n)});
        cursor += n;
    }
    if (cursor != ne) return 1;
    std::vector<Entry> entries;
    std::vector<CASC_CKEY_ENTRY> keys(ne + 6);
    for (std::size_t i = 0; i != ne; ++i) {
        auto id = input.number(4), key = input.number(4), hash = input.number(8), present = input.number(1);
        // This QA adapter bounds native sparse-array allocation by ID too.
        if (id > 100000 || key >= ne || present > 1) return 1;
        entries.push_back({std::uint32_t(id),std::uint32_t(key),hash,present != 0});
    }
    std::vector<std::string> queries;
    for (std::size_t i = 0; i != nq; ++i) queries.push_back(input.name());
    if (input.at != input.bytes.size()) return 1;
    Tree tree;
    // Feed rows in the audited WoW ROOT eligibility order. The actual selection,
    // late name assignment, retained CKey and hash lookup execute original code.
    for (unsigned audio = 0; audio != 2; ++audio) for (auto const& block : blocks) {
        if ((block.content >> 31) != audio || (block.content & 0x180) ||
            (block.locale && !(block.locale & locale))) continue;
        for (auto i = block.start; i != block.end; ++i) {
            auto const& row = entries[i];
            if (!row.present) continue;
            auto* node = row.hash ? tree.value.InsertByHash(&keys[row.key], row.hash, row.id, block.locale, block.content)
                                  : tree.value.InsertById(&keys[row.key], row.id, block.locale, block.content);
            if (!node) return 3;
        }
    }
    std::array<char const*,6> internal{"ENCODING","DOWNLOAD","INSTALL","PATCH","ROOT","SIZE"};
    for (std::size_t i=0; i<internal.size(); ++i)
        if (!tree.value.InsertByName(&keys[ne+i], internal[i])) return 3;
    for (auto const& query : queries) {
        auto hash = CalcFileNameHash(query.c_str());
        auto* node = tree.value.Find(hash);
        emit(hash, 8); emit(node != nullptr, 1);
        DWORD id = CASC_INVALID_ID;
        if (node) tree.value.GetExtras(node, &id, nullptr, nullptr);
        emit(id, 4); emit(node ? node->pCKeyEntry - keys.data() : UINT32_MAX, 4);
    }
    return std::fflush(stdout) ? 2 : 0;
} catch (...) { return 4; }
