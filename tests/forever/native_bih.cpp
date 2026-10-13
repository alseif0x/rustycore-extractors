// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only caller of the original pinned BIH.
// Reference: 02245dcd245e7433e524577656177723d3e4992e, unchanged
// src/common/Collision/BoundingIntervalHierarchy.{h,cpp} and G3D dependencies.
// All hierarchy construction and serialization, including its NaN/inf markers,
// are performed by the original implementation. No hierarchy algorithm or
// upstream support method is replaced here. Preserve original source notices.
//
// RCEBIH01: magic8, leaf_size:u32LE (1 or 3), count:u32LE (<=100000),
// then count * {low_x,low_y,low_z,high_x,high_y,high_z}:f32LE. The complete
// envelope is exact, bounded and contains only finite ordered synthetic boxes.
// CLI: fixture new-private-stage. Original build/write yields native.bih;
// original read/write yields roundtrip.bih. Both outputs are exclusive0600 in
// a newly created0700 stage. They must be byte-identical, including empty-tree
// NaNs and one-child infinity sentinels. No queries, acquisition or model parsing
// are exercised. Errors leave any already-created private stage for inspection.
//
// Build the pinned G3D release configuration (G3D_DEBUG absent). Empty BIH
// readFromFile constructs AABox from NaNs, which G3D debug assertions reject;
// no assertion hook or macro is overridden by this adapter.
#include "BoundingIntervalHierarchy.h"
#include <algorithm>
#include <array>
#include <bit>
#include <cmath>
#include <cstdio>
#include <filesystem>
#include <limits>
#include <memory>
#include <string_view>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#ifdef G3D_DEBUG
#error "native_bih empty roundtrip requires the original G3D release configuration"
#endif

namespace {
constexpr std::size_t MaxPrimitives = 100000;
constexpr std::size_t MaxInputBytes = 16 + 24 * MaxPrimitives;
constexpr std::size_t MaxOutputBytes = 256 * 1024 * 1024;
using Bytes = std::vector<unsigned char>;
using File = std::unique_ptr<FILE, int (*)(FILE*)>;

File open_file(std::filesystem::path const& path, bool create) {
    int fd = ::open(path.c_str(), create ? O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW
                                       : O_RDONLY | O_NOFOLLOW, 0600);
    if (fd < 0) throw "qa_file_open";
    FILE* file = ::fdopen(fd, create ? "wb" : "rb");
    if (!file) {
        ::close(fd);
        throw "qa_file_open";
    }
    return File(file, &std::fclose);
}

std::size_t regular_size(FILE* file, std::size_t maximum) {
    struct stat info {};
    if (::fstat(::fileno(file), &info) || !S_ISREG(info.st_mode)
        || info.st_size < 0 || static_cast<unsigned long long>(info.st_size) > maximum)
        throw "qa_extent";
    return static_cast<std::size_t>(info.st_size);
}

Bytes read_input(std::filesystem::path const& path) {
    auto file = open_file(path, false);
    auto size = regular_size(file.get(), MaxInputBytes);
    if (size < 16) throw "qa_extent";
    Bytes bytes(size);
    if (std::fread(bytes.data(), 1, size, file.get()) != size
        || std::fgetc(file.get()) != EOF || std::ferror(file.get()))
        throw "qa_input";
    return bytes;
}

uint32 word(Bytes const& bytes, std::size_t at) {
    return uint32(bytes.at(at)) | (uint32(bytes.at(at + 1)) << 8)
        | (uint32(bytes.at(at + 2)) << 16) | (uint32(bytes.at(at + 3)) << 24);
}

struct BoundsFunc {
    void operator()(G3D::AABox const& source, G3D::AABox& destination) const {
        destination = source;
    }
};

std::size_t write_tree(BIH const& tree, std::filesystem::path const& path) {
    auto file = open_file(path, true);
    if (!tree.writeToFile(file.get()) || std::fflush(file.get())
        || ::fsync(::fileno(file.get())))
        throw "qa_output";
    auto size = regular_size(file.get(), MaxOutputBytes);
    if (std::fclose(file.release())) throw "qa_output";
    return size;
}

void equal_files(std::filesystem::path const& first, std::filesystem::path const& second,
                 std::size_t expected) {
    auto a = open_file(first, false);
    auto b = open_file(second, false);
    if (regular_size(a.get(), MaxOutputBytes) != expected
        || regular_size(b.get(), MaxOutputBytes) != expected)
        throw "qa_roundtrip_extent";
    std::array<unsigned char, 4096> left {}, right {};
    for (std::size_t remaining = expected; remaining;) {
        auto count = std::min(remaining, left.size());
        if (std::fread(left.data(), 1, count, a.get()) != count
            || std::fread(right.data(), 1, count, b.get()) != count)
            throw "qa_roundtrip_read";
        if (!std::equal(left.begin(), left.begin() + count, right.begin()))
            throw "qa_roundtrip_bytes";
        remaining -= count;
    }
    if (std::fgetc(a.get()) != EOF || std::fgetc(b.get()) != EOF
        || std::ferror(a.get()) || std::ferror(b.get()))
        throw "qa_roundtrip_tail";
}
}

int main(int argc, char** argv) try {
    static_assert(std::endian::native == std::endian::little);
    static_assert(sizeof(float) == 4 && sizeof(uint32) == 4);
    static_assert(std::numeric_limits<float>::is_iec559);
    if (argc != 3) {
        std::puts("{\"status\":\"error\",\"code\":\"qa_usage\"}");
        return 2;
    }
    ::umask(0077);
    auto bytes = read_input(argv[1]);
    if (std::string_view(reinterpret_cast<char const*>(bytes.data()), 8) != "RCEBIH01")
        throw "qa_magic";
    auto leaf = word(bytes, 8);
    auto count = word(bytes, 12);
    if ((leaf != 1 && leaf != 3) || count > MaxPrimitives
        || bytes.size() != 16 + std::size_t(count) * 24)
        throw "qa_shape";
    // Validate all geometry before constructing native boxes or invoking build.
    for (std::size_t i = 0; i < count; ++i) {
        auto at = 16 + i * 24;
        for (std::size_t axis = 0; axis < 3; ++axis) {
            auto low = std::bit_cast<float>(word(bytes, at + axis * 4));
            auto high = std::bit_cast<float>(word(bytes, at + 12 + axis * 4));
            if (!std::isfinite(low) || !std::isfinite(high)) throw "qa_nonfinite";
            if (low > high) throw "qa_bounds";
        }
    }
    std::vector<G3D::AABox> boxes;
    boxes.reserve(count);
    for (std::size_t i = 0; i < count; ++i) {
        std::array<float, 6> values {};
        for (std::size_t axis = 0; axis < values.size(); ++axis)
            values[axis] = std::bit_cast<float>(word(bytes, 16 + i * 24 + axis * 4));
        boxes.emplace_back(G3D::Vector3(values[0], values[1], values[2]),
                           G3D::Vector3(values[3], values[4], values[5]));
    }
    BIH tree;
    tree.build(boxes, BoundsFunc{}, leaf, false);
    if (tree.primCount() != count) throw "qa_build_count";
    auto stage = std::filesystem::absolute(argv[2]);
    if (::mkdir(stage.c_str(), 0700)) throw "qa_stage_exists";
    auto native_path = stage / "native.bih";
    auto roundtrip_path = stage / "roundtrip.bih";
    auto output_size = write_tree(tree, native_path);
    BIH restored;
    {
        auto input = open_file(native_path, false);
        if (regular_size(input.get(), MaxOutputBytes) != output_size
            || !restored.readFromFile(input.get())
            || std::fgetc(input.get()) != EOF || std::ferror(input.get()))
            throw "qa_native_read";
    }
    if (restored.primCount() != count) throw "qa_roundtrip_count";
    if (write_tree(restored, roundtrip_path) != output_size) throw "qa_roundtrip_extent";
    equal_files(native_path, roundtrip_path, output_size);
    std::printf("{\"status\":\"ok\",\"primitives\":%u,\"leaf_size\":%u,\"bytes\":%zu,"
                "\"build_calls\":1,\"write_calls\":2,\"read_calls\":1,\"roundtrip_exact\":true}\n",
                count, leaf, output_size);
    return 0;
} catch (char const* code) {
    std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n", code);
    return 4;
} catch (...) {
    std::puts("{\"status\":\"error\",\"code\":\"qa_failure\"}");
    return 4;
}
