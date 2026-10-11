// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic GridMap height comparison kernel.
// Pinned algorithm/format facts: TrinityCore
// 02245dcd245e7433e524577656177723d3e4992e,
// src/server/game/Maps/GridMap.cpp:60-114,156-242,280-289,304-546;
// src/server/game/Grids/GridDefines.h:37-61;
// src/common/Collision/Maps/MapDefines.h:41-85 and MapDefines.cpp:20-24.
// Independently transcribed facts; no upstream implementation is imported.
// Upstream authors retain their copyrights under GPL-2.0-or-later.
//
// This is NOT the complete C++ GridMap, an engine or a production MAPS
// verifier. It reads only the file header, MHGT and holes used by getHeight.
// AREA, liquid, build and declared section sizes are deliberately ignored;
// referenced height arrays and the optional 36-byte MFBO tail must be present.
// Unrelated tails/overlaps/unknown flag bits are not whole-container validated.
// MFBO planes, area/liquid queries, engine loading and runtime integration are
// outside this kernel. Flat getHeight ignores holes, as in the pinned source.
// No finiteness policy is imposed on stored height bits or interpolation
// results; malformed rejection parity and portable NaN payloads are not claimed.
//
// Compile as C++20 without fast-math and with -ffp-contract=off. Arithmetic is
// binary32, preserving the pin's operation order and integer coefficients.
// CLI: oracle <map-synthetic.bin> <queries.bin> <output.bin>
// Queries: exact RCEQUERY (8 bytes), LE u32 count <=4096, then count pairs
// {x:f32LE,y:f32LE}; finite coordinates with absolute value <=20000.
// Map input <=202112 bytes. Output: count raw u32LE height bits, no header.
// POSIX fixture I/O only: exclusive output after all parsing/query evaluation,
// fsync before success; static status/counts on stdout, no paths or raw data.
// No client discovery, acquisition, network or production source mutation.

#include <algorithm>
#include <array>
#include <bit>
#include <cerrno>
#include <cfloat>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <limits>
#include <new>
#include <span>
#include <string_view>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#ifdef __FAST_MATH__
#error "The comparison kernel requires strict floating-point arithmetic"
#endif
static_assert(sizeof(float) == 4 && std::numeric_limits<float>::is_iec559);
static_assert(FLT_EVAL_METHOD == 0, "Binary32 intermediate evaluation is required");

namespace {
using Bytes = std::vector<std::uint8_t>;
using View = std::span<std::uint8_t const>;
constexpr std::size_t MapLimit = 202112;
constexpr std::uint32_t QueryLimit = 4096;
constexpr std::size_t CornerCount = 129 * 129;
constexpr std::size_t CenterCount = 128 * 128;
constexpr std::size_t HoleBytes = 16 * 16 * 8;
constexpr float MissingHeight = -100000.0f;
constexpr float GridWorldSize = 533.3333f;

struct Failure { char const* code; };
[[noreturn]] void fail(char const* code) { throw Failure{code}; }

class File {
public:
    explicit File(int descriptor) : descriptor_(descriptor) {}
    File(File const&) = delete;
    File& operator=(File const&) = delete;
    ~File() { if (descriptor_ >= 0) ::close(descriptor_); }
    int get() const { return descriptor_; }
    void close_output() {
        int descriptor = descriptor_;
        descriptor_ = -1;
        if (::close(descriptor) != 0)
            fail("qa_output_close");
    }
private:
    int descriptor_;
};

bool same_file(struct stat const& a, struct stat const& b) {
    return S_ISREG(a.st_mode) && S_ISREG(b.st_mode)
        && a.st_dev == b.st_dev && a.st_ino == b.st_ino && a.st_size == b.st_size
        && a.st_mtim.tv_sec == b.st_mtim.tv_sec && a.st_mtim.tv_nsec == b.st_mtim.tv_nsec
        && a.st_ctim.tv_sec == b.st_ctim.tv_sec && a.st_ctim.tv_nsec == b.st_ctim.tv_nsec;
}

Bytes read_fixture(char const* path, std::size_t limit) {
    File input(::open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK));
    if (input.get() < 0)
        fail("qa_input_open");
    struct stat before {};
    if (::fstat(input.get(), &before) != 0)
        fail("qa_input_metadata");
    if (!S_ISREG(before.st_mode) || before.st_size < 0)
        fail("qa_input_not_regular");
    if (static_cast<std::uintmax_t>(before.st_size) > limit)
        fail("qa_input_limit");
    Bytes data(static_cast<std::size_t>(before.st_size));
    std::size_t consumed = 0;
    while (consumed < data.size()) {
        auto n = ::read(input.get(), data.data() + consumed, data.size() - consumed);
        if (n < 0 && errno == EINTR)
            continue;
        if (n <= 0)
            fail("qa_input_read");
        consumed += static_cast<std::size_t>(n);
    }
    std::uint8_t extra = 0;
    ssize_t tail;
    do { tail = ::read(input.get(), &extra, 1); } while (tail < 0 && errno == EINTR);
    if (tail != 0)
        fail("qa_input_changed");
    struct stat after {}, named {};
    if (::fstat(input.get(), &after) != 0 || ::lstat(path, &named) != 0
        || !same_file(before, after) || !same_file(after, named))
        fail("qa_input_changed");
    return data;
}

View span_at(View bytes, std::size_t at, std::size_t size) {
    if (at > bytes.size() || size > bytes.size() - at)
        fail("qa_map_bounds");
    return bytes.subspan(at, size);
}
std::uint16_t u16(View data, std::size_t at) {
    auto p = span_at(data, at, 2);
    return std::uint16_t(p[0]) | (std::uint16_t(p[1]) << 8);
}
std::uint32_t u32(View data, std::size_t at) {
    auto p = span_at(data, at, 4);
    return std::uint32_t(p[0]) | (std::uint32_t(p[1]) << 8)
        | (std::uint32_t(p[2]) << 16) | (std::uint32_t(p[3]) << 24);
}
float f32(View data, std::size_t at) { return std::bit_cast<float>(u32(data, at)); }
bool tag(View data, std::size_t at, std::string_view expected) {
    auto actual = span_at(data, at, expected.size());
    return std::equal(expected.begin(), expected.end(), actual.begin());
}

struct Query { float x; float y; };
std::vector<Query> parse_queries(View bytes) {
    if (bytes.size() < 12 || !tag(bytes, 0, "RCEQUERY"))
        fail("qa_query_header");
    auto count = u32(bytes, 8);
    if (count > QueryLimit)
        fail("qa_query_limit");
    if (bytes.size() != 12 + std::size_t(count) * 8)
        fail("qa_query_extent");
    std::vector<Query> queries;
    queries.reserve(count);
    for (std::uint32_t i = 0; i < count; ++i) {
        Query query { f32(bytes, 12 + std::size_t(i) * 8), f32(bytes, 16 + std::size_t(i) * 8) };
        if (!std::isfinite(query.x) || !std::isfinite(query.y)
            || std::fabs(query.x) > 20000.0f || std::fabs(query.y) > 20000.0f)
            fail("qa_query_coordinate");
        queries.push_back(query);
    }
    return queries;
}

enum class Encoding { Flat, Float, U16, U8 };
struct HeightMap {
    Encoding encoding = Encoding::Flat;
    float base = MissingHeight;
    float step = 0.0f;
    View corners;
    View centers;
    View holes;
};

HeightMap parse_height(View file) {
    span_at(file, 0, 44);
    if (!tag(file, 0, "MAPS") || u32(file, 4) != 10)
        fail("qa_map_header");
    HeightMap result;
    auto height_at = u32(file, 20);
    if (height_at != 0) {
        auto header = span_at(file, height_at, 16);
        if (!tag(header, 0, "MHGT"))
            fail("qa_height_header");
        auto flags = u32(header, 4);
        result.base = f32(header, 8);
        std::size_t cursor = std::size_t(height_at) + 16;
        if ((flags & 1) == 0) {
            std::size_t width;
            if ((flags & 2) != 0) {
                result.encoding = Encoding::U16;
                width = 2;
                result.step = (f32(header, 12) - result.base) / 65535.0f;
            } else if ((flags & 4) != 0) {
                result.encoding = Encoding::U8;
                width = 1;
                result.step = (f32(header, 12) - result.base) / 255.0f;
            } else {
                result.encoding = Encoding::Float;
                width = 4;
            }
            result.corners = span_at(file, cursor, CornerCount * width);
            cursor += CornerCount * width;
            result.centers = span_at(file, cursor, CenterCount * width);
            cursor += CenterCount * width;
        }
        // getHeight does not use flight planes, but loadHeightData reads both
        // int16[9] arrays after any height encoding, including flat.
        if ((flags & 8) != 0)
            span_at(file, cursor, 36);
    }
    // Native loadData selects holes by holesSize, not holesOffset. The size
    // argument is otherwise ignored: the fixed 2048-byte array is required.
    if (u32(file, 40) != 0)
        result.holes = span_at(file, u32(file, 36), HoleBytes);
    return result;
}

struct Position { std::size_t row; std::size_t col; float x; float y; };
Position position(Query query) {
    float x = 128.0f * (32.0f - query.x / GridWorldSize);
    float y = 128.0f * (32.0f - query.y / GridWorldSize);
    // Bounded finite query coordinates make both int32 conversions defined.
    auto row = static_cast<std::int32_t>(x);
    auto col = static_cast<std::int32_t>(y);
    return { std::uint32_t(row) & 127u, std::uint32_t(col) & 127u,
             x - static_cast<float>(row), y - static_cast<float>(col) };
}
bool hole(HeightMap const& map, Position p) {
    if (map.holes.empty())
        return false;
    std::size_t byte = (p.row / 8) * 128 + (p.col / 8) * 8 + p.row % 8;
    return (map.holes[byte] & (1u << (p.col % 8))) != 0;
}

// Coefficients of the four planes through two corners and the center. The
// arithmetic order matters for float heights. Integer encodings derive these
// coefficients in int32 first (every result is within +/-131070), then convert
// each coefficient for the binary32 interpolation.
template<class Number>
std::array<Number, 3> coefficients(unsigned triangle, Number nw, Number sw,
                                  Number ne, Number se, Number doubled_center) {
    switch (triangle) {
        case 0: return { sw - nw, (doubled_center - nw) - sw, nw };
        case 1: return { (doubled_center - nw) - ne, ne - nw, nw };
        case 2: return { (sw + se) - doubled_center, se - sw, doubled_center - se };
        default: return { se - ne, (ne + se) - doubled_center, doubled_center - se };
    }
}
template<class Number>
float interpolate(std::array<Number, 3> plane, Position p) {
    float row_term = static_cast<float>(plane[0]) * p.x;
    float column_term = static_cast<float>(plane[1]) * p.y;
    float slope = row_term + column_term;
    return slope + static_cast<float>(plane[2]);
}
float height(HeightMap const& map, Query query) {
    if (map.encoding == Encoding::Flat)
        return map.base;
    auto p = position(query);
    if (hole(map, p))
        return MissingHeight;
    unsigned triangle = (p.x + p.y < 1.0f ? 0u : 2u) + (p.x > p.y ? 0u : 1u);
    std::size_t corner = p.row * 129 + p.col;
    std::size_t center = p.row * 128 + p.col;
    if (map.encoding == Encoding::Float) {
        auto value = [&](std::size_t index) { return f32(map.corners, index * 4); };
        auto plane = coefficients(triangle, value(corner), value(corner + 129),
                                  value(corner + 1), value(corner + 130),
                                  2.0f * f32(map.centers, center * 4));
        return interpolate(plane, p);
    }
    auto integer = [&](View data, std::size_t index) -> std::int32_t {
        return map.encoding == Encoding::U16 ? u16(data, index * 2) : data[index];
    };
    auto plane = coefficients(triangle, integer(map.corners, corner),
                              integer(map.corners, corner + 129),
                              integer(map.corners, corner + 1),
                              integer(map.corners, corner + 130),
                              2 * integer(map.centers, center));
    float normalized = interpolate(plane, p);
    float scaled = normalized * map.step;
    return scaled + map.base;
}

void append_word(Bytes& bytes, std::uint32_t word) {
    for (unsigned shift = 0; shift < 32; shift += 8)
        bytes.push_back(static_cast<std::uint8_t>(word >> shift));
}
void write_output(char const* path, View bytes) {
    File out(::open(path, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC | O_NOFOLLOW, 0600));
    if (out.get() < 0)
        fail("qa_output_create");
    std::size_t cursor = 0;
    while (cursor < bytes.size()) {
        auto n = ::write(out.get(), bytes.data() + cursor, bytes.size() - cursor);
        if (n < 0 && errno == EINTR)
            continue;
        if (n <= 0)
            fail("qa_output_write");
        cursor += static_cast<std::size_t>(n);
    }
    if (::fsync(out.get()) != 0)
        fail("qa_output_sync");
    out.close_output();
    // I/O failure can leave a partial tool-owned file. It is never reported
    // successful and is not removed or replaced automatically.
}
int report_error(char const* code, int status) {
    std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n", code);
    std::fflush(stdout);
    return status;
}
} // namespace

int main(int argc, char** argv) {
    if (argc != 4)
        return report_error("qa_invocation", 2);
    try {
        auto map_bytes = read_fixture(argv[1], MapLimit);
        auto query_bytes = read_fixture(argv[2], 12 + std::size_t(QueryLimit) * 8);
        auto map = parse_height(map_bytes);
        auto queries = parse_queries(query_bytes);
        Bytes output;
        output.reserve(queries.size() * 4);
        for (auto query : queries)
            append_word(output, std::bit_cast<std::uint32_t>(height(map, query)));
        write_output(argv[3], output);
        if (std::printf("{\"status\":\"success\",\"queries\":%zu}\n", queries.size()) < 0
            || std::fflush(stdout) != 0)
            return 4;
        return 0;
    } catch (Failure const& error) {
        return report_error(error.code, 4);
    } catch (std::bad_alloc const&) {
        return report_error("qa_allocation_limit", 4);
    } catch (...) {
        return report_error("qa_internal", 4);
    }
}
