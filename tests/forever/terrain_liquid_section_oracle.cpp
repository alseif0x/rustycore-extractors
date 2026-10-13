// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic MLIQ packing comparison kernel.
// Format/algorithm reference: TrinityCore commit
// 02245dcd245e7433e524577656177723d3e4992e,
// src/tools/map_extractor/System.cpp:103-112,868-955,1014-1026 and
// src/common/Collision/Maps/MapDefines.h:87-130, MapDefines.cpp:24.
// Independently written from those facts; no upstream implementation imported.
// Upstream remains copyrighted by its authors under GPL-2.0-or-later.
//
// Synthetic QA only, not the pinned executable or a MAPS/ADT/DB2 loader.
// The exact input shape, boolean visibility, all-height finite validation and
// caller size cap are additional safe admission matching the Rust section API;
// they do not establish original C++ rejection behavior. Numerical processing
// preserves source float order, signed-zero choices, sentinels, invisible-cell
// replacement, empty unsigned extents and row-major crop serialization.
// Compile as C++20 with -ffp-contract=off, without fast-math.
//
// CLI: oracle <input-synthetic.bin> <output.bin>
// Input: RCELS001 + pack u8 (0|1) + cap u32 LE + show[16384] u8 +
// entries[256] u16 LE + flags[256] u8 + heightBits[16641] u32 LE.
// Exactly 83729 bytes. Output is raw MLIQ, or a new empty file on omission.
// No client discovery, CASC, network, integration reader or full MAPS output.

#include <algorithm>
#include <array>
#include <bit>
#include <cerrno>
#include <cmath>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <limits>
#include <optional>
#include <span>
#include <string_view>
#include <vector>
#include <fcntl.h>
#include <unistd.h>

namespace {
using Bytes = std::vector<std::uint8_t>;
using View = std::span<std::uint8_t const>;
constexpr std::size_t Grid = 128;
constexpr std::size_t ShowCount = Grid * Grid;
constexpr std::size_t CellCount = 256;
constexpr std::size_t HeightCount = (Grid + 1) * (Grid + 1);
constexpr std::size_t EntryStart = 13 + ShowCount;
constexpr std::size_t FlagStart = EntryStart + CellCount * 2;
constexpr std::size_t HeightStart = FlagStart + CellCount;
constexpr std::size_t InputBytes = HeightStart + HeightCount * 4;
constexpr std::size_t SectionLimit = 16 + CellCount * 3 + HeightCount * 4;
static_assert(InputBytes == 83729 && SectionLimit == 67348);
static_assert(sizeof(float) == 4 && std::numeric_limits<float>::is_iec559);

struct Failure { char const* code; };
[[noreturn]] void fail(char const* code) { throw Failure{code}; }

std::uint16_t short_at(View data, std::size_t at) {
    return std::uint16_t(data[at]) | (std::uint16_t(data[at + 1]) << 8);
}

std::uint32_t word_at(View data, std::size_t at) {
    return std::uint32_t(data[at]) | (std::uint32_t(data[at + 1]) << 8)
        | (std::uint32_t(data[at + 2]) << 16) | (std::uint32_t(data[at + 3]) << 24);
}

void append_le(Bytes& bytes, std::uint32_t value, unsigned width) {
    for (unsigned i = 0; i < width; ++i)
        bytes.push_back(static_cast<std::uint8_t>(value >> (i * 8)));
}

Bytes read_input(char const* path) {
    std::error_code ec;
    if (!std::filesystem::is_regular_file(path, ec) || ec)
        fail("qa_input_not_file");
    auto size = std::filesystem::file_size(path, ec);
    if (ec)
        fail("qa_input_metadata");
    if (size != InputBytes)
        fail("qa_input_extent");
    std::ifstream input(path, std::ios::binary);
    if (!input)
        fail("qa_input_open");
    Bytes bytes(InputBytes);
    if (!input.read(reinterpret_cast<char*>(bytes.data()), static_cast<std::streamsize>(bytes.size())))
        fail("qa_input_read");
    char extra = 0;
    if (input.get(extra) || !input.eof())
        fail("qa_input_read");
    return bytes;
}

std::optional<Bytes> section(View data) {
    if (data.size() != InputBytes)
        fail("qa_input_extent");
    constexpr std::string_view magic = "RCELS001";
    if (!std::equal(magic.begin(), magic.end(), data.begin()))
        fail("qa_input_header");
    if (data[8] > 1)
        fail("qa_input_option");
    bool pack = data[8] != 0;
    auto cap = word_at(data, 9);
    auto show = data.subspan(13, ShowCount);
    auto flags = data.subspan(FlagStart, CellCount);
    std::array<std::uint16_t, CellCount> types;
    std::array<float, HeightCount> heights;
    for (auto value : show)
        if (value > 1)
            fail("liquid_section_invalid_visibility");
    for (std::size_t i = 0; i < CellCount; ++i)
        types[i] = short_at(data, EntryStart + i * 2);
    for (std::size_t i = 0; i < HeightCount; ++i) {
        heights[i] = std::bit_cast<float>(word_at(data, HeightStart + i * 4));
        if (!std::isfinite(heights[i]))
            fail("liquid_section_nonfinite_height");
    }

    bool full_type = false;
    for (std::size_t i = 0; i < CellCount; ++i)
        if (types[i] != types.front() || flags[i] != flags.front()) {
            full_type = true;
            break;
        }
    if (!full_type && flags.front() == 0)
        return std::nullopt;

    int left = 128, top = 128, right = 0, bottom = 0;
    float low = 20000.0f, high = -20000.0f;
    // Like the source, replace invisible interior samples in the matrix before
    // writing the crop. Its final row and column remain original input values.
    for (int row = 0; row < 128; ++row) {
        for (int column = 0; column < 128; ++column) {
            auto at = static_cast<std::size_t>(row * 129 + column);
            if (show[static_cast<std::size_t>(row * 128 + column)] != 0) {
                if (column < left) left = column;
                if (column > right) right = column;
                if (row < top) top = row;
                if (row > bottom) bottom = row;
                if (heights[at] > high) high = heights[at];
                if (heights[at] < low) low = heights[at];
            } else {
                heights[at] = -2000.0f;
                if (low > -2000.0f) low = -2000.0f;
            }
        }
    }
    std::uint8_t header_flags = 0;
    if (top > bottom || left > right)
        header_flags |= 2;
    if (high == low)
        header_flags |= 2;
    if (pack && high - low < 0.001f)
        header_flags |= 2;
    if (!full_type)
        header_flags |= 1;
    auto width = static_cast<std::uint8_t>(right - left + 2);
    auto height = static_cast<std::uint8_t>(bottom - top + 2);
    std::size_t size = 16 + (full_type ? CellCount * 3 : 0);
    if ((header_flags & 2) == 0)
        size += std::size_t(width) * height * 4;
    if (size > cap || size > SectionLimit)
        fail("liquid_section_policy_limit");

    Bytes bytes;
    bytes.reserve(size);
    for (char value : std::string_view("MLIQ"))
        bytes.push_back(static_cast<std::uint8_t>(value));
    bytes.push_back(header_flags);
    bytes.push_back(full_type ? 0 : flags.front());
    append_le(bytes, full_type ? 0 : types.front(), 2);
    bytes.push_back(static_cast<std::uint8_t>(left));
    bytes.push_back(static_cast<std::uint8_t>(top));
    bytes.push_back(width);
    bytes.push_back(height);
    append_le(bytes, std::bit_cast<std::uint32_t>(low), 4);
    if (full_type) {
        for (auto value : types)
            append_le(bytes, value, 2);
        bytes.insert(bytes.end(), flags.begin(), flags.end());
    }
    if ((header_flags & 2) == 0) {
        for (int y = 0; y < height; ++y)
            for (int x = 0; x < width; ++x) {
                auto at = static_cast<std::size_t>((top + y) * 129 + left + x);
                append_le(bytes, std::bit_cast<std::uint32_t>(heights[at]), 4);
            }
    }
    if (bytes.size() != size)
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
    if (argc != 3) {
        std::cout << "{\"status\":\"error\",\"code\":\"qa_usage\"}\n";
        return 2;
    }
    try {
        auto input = read_input(argv[1]);
        auto output = section(input);
        View bytes = output ? View(*output) : View{};
        write_new(argv[2], bytes);
        std::cout << "{\"status\":\"" << (output ? "ok" : "omit")
                  << "\",\"bytes\":" << bytes.size() << "}\n";
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
