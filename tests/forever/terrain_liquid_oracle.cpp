// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic liquid-projection comparison kernel.
// Format/algorithm reference: TrinityCore commit
// 02245dcd245e7433e524577656177723d3e4992e,
// src/tools/map_extractor/adt.h:60-81,148-296 and
// src/tools/map_extractor/System.cpp:430-446,491-494,577-630,647-713.
// Independently written from those facts; no upstream implementation imported.
// Upstream remains copyrighted by its authors under GPL-2.0-or-later.
//
// This is a synthetic QA kernel, not the pinned executable or its byte-scanning
// GetSubChunk loader. Sequential framing, duplicate checks, bounds, rectangles,
// a 16 MiB input ceiling and 4096 aggregate frames are explicit QA admission.
// The numeric projection follows the reference: old liquid before new liquid,
// last height/entry assignment, accumulated visibility/flags, first MH2O layer.
// Unlike Rust's stricter projection, unknown selected vertex formats yield zero
// and non-finite height bits are retained. Do not claim native rejection parity
// for malformed input or NaN payload compatibility from this kernel.
//
// CLI: oracle <adt-synthetic.bin> <lookup-synthetic.bin> <0|1> <output.bin>
// Lookup: RCELIQ01 + LE u32 counts, types {u32,u8,u8}, materials {u32,u32,i8},
// <=256 records each, exact EOF and last duplicate ID wins. Output: RCELQP01 +
// show[16384] + LE u16 entries[256] + flags[256] + LE u32 heightBits[16641].
// No client discovery, CASC, network, DB2 loading or MLIQ packing occurs.

#include <algorithm>
#include <array>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <limits>
#include <map>
#include <optional>
#include <span>
#include <string_view>
#include <vector>
#include <fcntl.h>
#include <unistd.h>

namespace {
using Bytes = std::vector<std::uint8_t>;
using View = std::span<std::uint8_t const>;
constexpr std::size_t AdtLimit = 16 * 1024 * 1024;
constexpr std::size_t LookupLimit = 16 + 256 * 6 + 256 * 9;
constexpr std::size_t FrameLimit = 4096;
constexpr std::size_t OutputBytes = 83724;

struct Failure { char const* code; };
[[noreturn]] void fail(char const* code) { throw Failure{code}; }

View extent(View data, std::size_t offset, std::size_t count, char const* code = "liquid_bounds") {
    if (offset > data.size() || count > data.size() - offset)
        fail(code);
    return data.subspan(offset, count);
}

std::uint16_t short_at(View data, std::size_t offset) {
    auto p = extent(data, offset, 2);
    return std::uint16_t(p[0]) | (std::uint16_t(p[1]) << 8);
}

std::uint32_t word_at(View data, std::size_t offset) {
    auto p = extent(data, offset, 4);
    return std::uint32_t(p[0]) | (std::uint32_t(p[1]) << 8)
        | (std::uint32_t(p[2]) << 16) | (std::uint32_t(p[3]) << 24);
}

std::uint64_t long_at(View data, std::size_t offset) {
    return std::uint64_t(word_at(data, offset)) | (std::uint64_t(word_at(data, offset + 4)) << 32);
}

Bytes read_bounded(char const* path, std::size_t limit) {
    std::error_code ec;
    if (!std::filesystem::is_regular_file(path, ec) || ec)
        fail("qa_input_not_file");
    auto size = std::filesystem::file_size(path, ec);
    if (ec)
        fail("qa_input_metadata");
    if (size > limit)
        fail("qa_input_limit");
    std::ifstream input(path, std::ios::binary);
    if (!input)
        fail("qa_input_open");
    Bytes bytes(static_cast<std::size_t>(size));
    if (!bytes.empty() && !input.read(reinterpret_cast<char*>(bytes.data()), static_cast<std::streamsize>(size)))
        fail("qa_input_read");
    char extra = 0;
    if (input.get(extra) || !input.eof())
        fail("qa_input_read");
    return bytes;
}

struct Type { std::uint8_t sound; std::uint8_t material; };
struct Material { std::uint32_t flags; std::int16_t signed_format; };
struct Lookup {
    std::map<std::uint32_t, Type> types;
    std::map<std::uint32_t, Material> materials;
};

Lookup parse_lookup(View data) {
    constexpr std::string_view magic = "RCELIQ01";
    if (data.size() < 16 || !std::equal(magic.begin(), magic.end(), data.begin()))
        fail("qa_lookup_header");
    auto types = word_at(data, 8);
    auto materials = word_at(data, 12);
    if (types > 256 || materials > 256)
        fail("qa_lookup_limit");
    if (data.size() != 16 + types * 6 + materials * 9)
        fail("qa_lookup_extent");
    Lookup lookup;
    std::size_t cursor = 16;
    for (std::uint32_t i = 0; i < types; ++i, cursor += 6)
        lookup.types[word_at(data, cursor)] = Type{data[cursor + 4], data[cursor + 5]};
    for (std::uint32_t i = 0; i < materials; ++i, cursor += 9) {
        auto raw = data[cursor + 8];
        auto signed_format = static_cast<std::int16_t>(raw < 128 ? raw : int(raw) - 256);
        lookup.materials[word_at(data, cursor)] = Material{word_at(data, cursor + 4), signed_format};
    }
    return lookup;
}

struct Frame {
    std::array<char, 4> tag;
    View payload;
    bool is(std::string_view wanted) const {
        return wanted.size() == tag.size() && std::equal(tag.begin(), tag.end(), wanted.begin());
    }
};

std::vector<Frame> frames(View data, std::size_t budget) {
    std::vector<Frame> result;
    result.reserve(std::min(budget, data.size() / 8));
    std::size_t cursor = 0;
    while (cursor < data.size()) {
        if (result.size() == budget)
            fail("terrain_policy_limit");
        auto header = extent(data, cursor, 8, "terrain_chunk_truncated");
        auto count = word_at(header, 4);
        auto body = extent(data, cursor + 8, count, "terrain_chunk_truncated");
        result.push_back(Frame{{char(header[3]), char(header[2]), char(header[1]), char(header[0])}, body});
        cursor += 8 + count;
    }
    return result;
}

std::optional<View> unique(std::vector<Frame> const& frames, std::string_view tag, char const* code) {
    std::optional<View> result;
    for (auto const& frame : frames) {
        if (frame.is(tag)) {
            if (result)
                fail(code);
            result = frame.payload;
        }
    }
    return result;
}

struct Cell { View header; std::vector<Frame> children; };
struct Adt { std::vector<Frame> top; std::vector<Cell> cells; };

Adt parse_adt(View data) {
    if (data.size() > AdtLimit)
        fail("terrain_policy_limit");
    Adt adt{frames(data, FrameLimit), {}};
    auto version = unique(adt.top, "MVER", "terrain_duplicate_chunk");
    if (!version)
        fail("terrain_missing_chunk");
    if (version->size() != 4)
        fail("terrain_invalid_chunk_size");
    if (word_at(*version, 0) != 18)
        fail("terrain_unsupported_version");
    auto remaining = FrameLimit - adt.top.size();
    for (auto const& frame : adt.top) {
        if (!frame.is("MCNK"))
            continue;
        auto header = extent(frame.payload, 0, 128, "terrain_invalid_chunk_size");
        if (word_at(header, 4) >= 16 || word_at(header, 8) >= 16)
            fail("terrain_invalid_cell_coordinates");
        std::vector<Frame> children;
        try {
            children = frames(frame.payload.subspan(128), remaining);
        } catch (Failure error) {
            if (std::string_view(error.code) == "terrain_policy_limit")
                throw;
            fail("terrain_unsupported_nested_layout");
        }
        remaining -= children.size();
        if (auto mcvt = unique(children, "MCVT", "terrain_duplicate_chunk"))
            if (mcvt->size() != 580)
                fail("terrain_invalid_chunk_size");
        adt.cells.push_back(Cell{header, std::move(children)});
    }
    return adt;
}

struct Projection {
    std::array<std::uint8_t, 128 * 128> show{};
    std::array<std::uint16_t, 16 * 16> entries{};
    std::array<std::uint8_t, 16 * 16> flags{};
    std::array<std::uint32_t, 129 * 129> heights{};
};

void old_liquid(Projection& out, Cell const& cell, bool ignore) {
    if (word_at(cell.header, 100) <= 8)
        return;
    auto selected = unique(cell.children, "MCLQ", "liquid_duplicate_chunk");
    if (!selected)
        return;
    auto liquid = extent(*selected, 0, 804);
    auto column = word_at(cell.header, 4);
    auto row = word_at(cell.header, 8);
    auto index = row * 16 + column;
    for (std::size_t ordinal = 0; ordinal != 64; ++ordinal) {
        auto marker = liquid[656 + ordinal];
        if (marker == 15)
            continue;
        out.show[(row * 8 + ordinal / 8) * 128 + column * 8 + ordinal % 8] = 1;
        if (!ignore && (marker & 128) != 0)
            out.flags[index] |= 16;
    }
    auto flags = word_at(cell.header, 0);
    for (std::uint16_t kind = 1; kind != 4; ++kind) {
        if ((flags & (std::uint32_t(1) << (kind + 1))) != 0) {
            out.entries[index] = kind;
            out.flags[index] |= std::uint8_t(1 << (kind - 1));
        }
    }
    for (std::size_t ordinal = 0; ordinal != 81; ++ordinal)
        out.heights[(row * 8 + ordinal / 9) * 129 + column * 8 + ordinal % 9]
            = word_at(liquid, 12 + ordinal * 8);
}

std::uint16_t vertex_format(View instance, Lookup const& lookup) {
    auto declared = short_at(instance, 2);
    if (declared < 42)
        return declared;
    auto type = short_at(instance, 0);
    if (type == 2)
        return 2;
    auto found_type = lookup.types.find(type);
    if (found_type != lookup.types.end()) {
        auto material = lookup.materials.find(found_type->second.material);
        if (material != lookup.materials.end())
            return static_cast<std::uint16_t>(material->second.signed_format);
    }
    return std::numeric_limits<std::uint16_t>::max();
}

void new_liquid(Projection& out, View liquid, Lookup const& lookup, bool ignore) {
    extent(liquid, 0, 256 * 12);
    for (std::size_t index = 0; index != 256; ++index) {
        auto offset = word_at(liquid, index * 12);
        if (offset == 0 || word_at(liquid, index * 12 + 4) == 0)
            continue;
        auto instance = extent(liquid, offset, 24);
        auto format = vertex_format(instance, lookup);
        auto type = format == 2 ? std::uint16_t(2) : short_at(instance, 0);
        out.entries[index] = type;
        auto found_type = lookup.types.find(type);
        if (found_type == lookup.types.end())
            continue;
        auto material = lookup.materials.find(found_type->second.material);
        if (material != lookup.materials.end() && (material->second.flags & 2) != 0)
            continue;

        auto attributes = word_at(liquid, index * 12 + 8);
        auto deep = std::numeric_limits<std::uint64_t>::max();
        if (attributes != 0)
            deep = long_at(extent(liquid, attributes, 16), 8);

        // Rectangle selection depends on the raw format, not the resolved one.
        auto ordinary = short_at(instance, 2) < 42;
        std::size_t x = ordinary ? instance[12] : 0;
        std::size_t y = ordinary ? instance[13] : 0;
        std::size_t width = ordinary ? instance[14] : 8;
        std::size_t height = ordinary ? instance[15] : 8;
        if (x + width > 8 || y + height > 8)
            fail("liquid_invalid_rectangle");
        auto bitmap = word_at(instance, 16);
        auto bits = bitmap == 0 ? std::numeric_limits<std::uint64_t>::max()
            : long_at(liquid, bitmap);
        auto x0 = index % 16 * 8 + x;
        auto y0 = index / 16 * 8 + y;
        for (std::size_t row = 0; row < height; ++row)
            for (std::size_t column = 0; column < width; ++column) {
                if ((bits & 1) != 0)
                    out.show[(y0 + row) * 128 + x0 + column] = 1;
                bits >>= 1;
            }

        auto sound = found_type->second.sound;
        if (sound < 4)
            out.flags[index] |= std::uint8_t(1 << sound);
        if (sound == 1 && !ignore && deep != 0)
            out.flags[index] |= 16;

        auto vertices = word_at(instance, 20);
        auto point_count = (width + 1) * (height + 1);
        std::size_t stride = format == 4 || format == 5 ? 8 : 4;
        auto uses_vertices = vertices != 0 && format <= 5 && format != 2;
        View values;
        if (uses_vertices)
            values = extent(liquid, vertices, point_count * stride);
        for (std::size_t ordinal = 0; ordinal < point_count; ++ordinal) {
            auto value = uses_vertices ? word_at(values, ordinal * stride + (stride == 8 ? 4 : 0)) : 0;
            out.heights[(y0 + ordinal / (width + 1)) * 129 + x0 + ordinal % (width + 1)] = value;
        }
    }
}

Projection project(Adt const& adt, Lookup const& lookup, bool ignore) {
    Projection out;
    for (auto const& cell : adt.cells)
        old_liquid(out, cell, ignore);
    if (auto liquid = unique(adt.top, "MH2O", "liquid_duplicate_chunk"))
        new_liquid(out, *liquid, lookup, ignore);
    return out;
}

void append_le(Bytes& bytes, std::uint32_t value, unsigned count) {
    for (unsigned i = 0; i != count; ++i)
        bytes.push_back(static_cast<std::uint8_t>(value >> (i * 8)));
}

Bytes serialize(Projection const& out) {
    Bytes bytes;
    bytes.reserve(OutputBytes);
    constexpr std::string_view magic = "RCELQP01";
    bytes.insert(bytes.end(), magic.begin(), magic.end());
    bytes.insert(bytes.end(), out.show.begin(), out.show.end());
    for (auto value : out.entries)
        append_le(bytes, value, 2);
    bytes.insert(bytes.end(), out.flags.begin(), out.flags.end());
    for (auto value : out.heights)
        append_le(bytes, value, 4);
    if (bytes.size() != OutputBytes)
        fail("qa_output_shape");
    return bytes;
}

void write_new(char const* path, View bytes) {
    int fd = ::open(path, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0600);
    if (fd < 0)
        fail("qa_output_create");
    std::size_t written = 0;
    while (written < bytes.size()) {
        auto count = ::write(fd, bytes.data() + written, bytes.size() - written);
        if (count < 0 && errno == EINTR)
            continue;
        if (count <= 0) {
            ::close(fd);
            fail("qa_output_write");
        }
        written += static_cast<std::size_t>(count);
    }
    auto synced = ::fsync(fd);
    auto closed = ::close(fd);
    if (synced != 0 || closed != 0)
        fail("qa_output_write");
}
} // namespace

int main(int argc, char** argv) {
    if (argc != 5 || (std::string_view(argv[3]) != "0" && std::string_view(argv[3]) != "1")) {
        std::cout << "{\"status\":\"error\",\"code\":\"qa_usage\"}\n";
        return 2;
    }
    try {
        auto bytes = read_bounded(argv[1], AdtLimit);
        auto lookup_bytes = read_bounded(argv[2], LookupLimit);
        auto lookup = parse_lookup(lookup_bytes);
        auto adt = parse_adt(bytes);
        auto result = project(adt, lookup, std::string_view(argv[3]) == "1");
        auto encoded = serialize(result);
        write_new(argv[4], encoded);
        std::cout << "{\"status\":\"ok\",\"show\":16384,\"entries\":256,\"flags\":256,\"heights\":16641,\"bytes\":83724}\n";
        return 0;
    } catch (Failure error) {
        std::cout << "{\"status\":\"error\",\"code\":\"" << error.code << "\"}\n";
    } catch (std::bad_alloc const&) {
        std::cout << "{\"status\":\"error\",\"code\":\"qa_allocation_limit\"}\n";
    } catch (...) {
        std::cout << "{\"status\":\"error\",\"code\":\"qa_internal_error\"}\n";
    }
    return 4;
}
