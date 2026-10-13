// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic QA adapter, not an extractor.
// Calls externally materialized, unmodified TrinityCore model.cpp at
// 02245dcd245e7433e524577656177723d3e4992e. Original copyrights and the
// GPL-2.0-or-later notice remain in those external sources; this repository
// does not import them. The build driver must hash all transitive sources and
// headers and retain their AUTHORS/COPYING and third-party license notices.
//
// CLI: native_m2_conversion <fixture> <output>
// Synthetic fixture: RCE2RAW1 (8 bytes), LE u32 vertex/index counts, six LE
// f32 bit patterns (minimum xyz, maximum xyz), LE u32 output cap. Header: 44
// bytes. Follow with vertex_count triples of LE f32 bits, then index_count
// LE u32 indices. Exact EOF. Indices are source u16 values promoted to u32.
//
// Finite values, ordered bounds and valid triangle indices are explicit QA
// admission constraints, not claims about the original model parser. Never
// call Model::open or supply client assets. Preparing the model calls original
// fixCoordSystem once per source vertex, matching Model::open; the one call to
// original ConvertToVMAPModel reverses that coordinate transform and swaps
// triangle indices. Signed zero is retained. No writer function is replaced.
//
// Compile this TU and the original model.cpp separately with C++20,
// -ffunction-sections -fdata-sections -Wl,--gc-sections,
// -fno-fast-math -ffp-contract=off. No CASC/G3D implementation or stub is needed.
// Inspect retained symbols to confirm Model::open, acquisition and Doodad
// code have been discarded. Original INDX declares 4 + 2*index_count bytes
// while writing a u32 count plus u32 indices; this adapter does not repair it.
//
// Original stdio is captured in a retained private stage log. The original
// writer opens only a fixed filename inside that fresh 0700 directory. Its
// complete output is checked, fsynced and published by exclusive hard link.
// Existing destinations are never opened for writing. Stages are retained.

#include <algorithm>
#include <array>
#include <bit>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <limits>
#include <new>
#include <span>
#include <string>
#include <string_view>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include "model.h"
#include "vmapexport.h"

namespace {
using Bytes = std::vector<std::uint8_t>;
using View = std::span<std::uint8_t const>;
constexpr std::size_t InputLimit = 64 * 1024 * 1024;
constexpr std::uint32_t VertexLimit = 1000000;
constexpr std::uint32_t IndexLimit = 3000000;
constexpr std::size_t OutputLimit = 32 * 1024 * 1024;
static_assert(std::endian::native == std::endian::little);
static_assert(sizeof(float) == 4 && std::numeric_limits<float>::is_iec559);
static_assert(sizeof(int) == 4 && sizeof(unsigned short) == 2);
static_assert(sizeof(uint32) == 4 && sizeof(ModelFlags) == 4);
static_assert(sizeof(Vec3D) == 12 && sizeof(AaBox3D) == 24);

struct Failure { char const* code; };
[[noreturn]] void fail(char const* code) { throw Failure{code}; }

class Fd {
public:
    explicit Fd(int value = -1) : value_(value) {}
    ~Fd() { if (value_ >= 0) ::close(value_); }
    Fd(Fd const&) = delete;
    Fd& operator=(Fd const&) = delete;
    int get() const { return value_; }
private:
    int value_;
};

bool unchanged(struct stat const& first, struct stat const& second) {
    return first.st_dev == second.st_dev && first.st_ino == second.st_ino
        && first.st_mode == second.st_mode && first.st_size == second.st_size
        && first.st_mtim.tv_sec == second.st_mtim.tv_sec
        && first.st_mtim.tv_nsec == second.st_mtim.tv_nsec
        && first.st_ctim.tv_sec == second.st_ctim.tv_sec
        && first.st_ctim.tv_nsec == second.st_ctim.tv_nsec;
}

Bytes read_complete(char const* path) {
    Fd fd(::open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK));
    if (fd.get() < 0)
        fail("qa_input_open");
    struct stat before {}, after {}, named {};
    if (::fstat(fd.get(), &before) != 0 || !S_ISREG(before.st_mode))
        fail("qa_input_not_file");
    if (before.st_size < 0 || static_cast<std::uintmax_t>(before.st_size) > InputLimit)
        fail("qa_input_limit");
    Bytes bytes(static_cast<std::size_t>(before.st_size));
    std::size_t offset = 0;
    while (offset < bytes.size()) {
        auto got = ::read(fd.get(), bytes.data() + offset, bytes.size() - offset);
        if (got < 0 && errno == EINTR)
            continue;
        if (got <= 0)
            fail("qa_input_read");
        offset += static_cast<std::size_t>(got);
    }
    std::uint8_t tail = 0;
    ssize_t extra;
    do { extra = ::read(fd.get(), &tail, 1); } while (extra < 0 && errno == EINTR);
    if (extra != 0)
        fail("qa_input_read");
    if (::fstat(fd.get(), &after) != 0 || ::lstat(path, &named) != 0
        || !unchanged(before, after) || !unchanged(before, named))
        fail("qa_input_changed");
    return bytes;
}

std::uint32_t word(View bytes, std::size_t offset) {
    if (offset > bytes.size() || bytes.size() - offset < 4)
        fail("qa_input_extent");
    return std::uint32_t(bytes[offset]) | (std::uint32_t(bytes[offset + 1]) << 8)
        | (std::uint32_t(bytes[offset + 2]) << 16)
        | (std::uint32_t(bytes[offset + 3]) << 24);
}

bool magic(View bytes, std::string_view expected) {
    return bytes.size() >= expected.size()
        && std::equal(expected.begin(), expected.end(), bytes.begin());
}

bool finite(std::uint32_t bits) { return (bits & 0x7f800000u) != 0x7f800000u; }

struct Fixture {
    View bytes;
    std::uint32_t vertices;
    std::uint32_t indices;
    std::array<float, 6> bounds;
    std::size_t indices_offset;
    std::size_t output_bytes;
    std::uint32_t output_cap;
};

Fixture fixture(View bytes) {
    if (bytes.size() < 44 || !magic(bytes, "RCE2RAW1"))
        fail("qa_input_format");
    Fixture value {bytes, word(bytes, 8), word(bytes, 12), {}, 0, 0, word(bytes, 40)};
    if (value.vertices == 0 || value.vertices > VertexLimit || value.indices == 0
        || value.indices > IndexLimit || value.indices % 3 != 0)
        fail("qa_input_counts");
    // Counts are bounded before either arithmetic or allocation. The largest
    // allowed fixture and output fit in 32 bits as well as in size_t.
    value.indices_offset = 44 + std::size_t(value.vertices) * 12;
    if (bytes.size() != value.indices_offset + std::size_t(value.indices) * 4)
        fail("qa_input_extent");
    value.output_bytes = 100 + std::size_t(value.indices) * 4
        + std::size_t(value.vertices) * 12;
    if (value.output_cap > OutputLimit || value.output_bytes > value.output_cap)
        fail("qa_output_limit");
    for (std::size_t i = 0; i < value.bounds.size(); ++i) {
        auto bits = word(bytes, 16 + i * 4);
        if (!finite(bits))
            fail("qa_nonfinite_bounds");
        value.bounds[i] = std::bit_cast<float>(bits);
    }
    for (std::size_t axis = 0; axis < 3; ++axis)
        if (value.bounds[axis] > value.bounds[axis + 3])
            fail("qa_bounds_order");
    for (std::size_t at = 44; at < value.indices_offset; at += 4)
        if (!finite(word(bytes, at)))
            fail("qa_nonfinite_vertex");
    for (std::size_t at = value.indices_offset; at < bytes.size(); at += 4) {
        auto index = word(bytes, at);
        if (index > std::numeric_limits<std::uint16_t>::max() || index >= value.vertices)
            fail("qa_index_range");
    }
    return value;
}

void prepare(Model& model, Fixture const& value) {
    model.header.nBoundingVertices = value.vertices;
    model.header.nBoundingTriangles = value.indices;
    model.bounds.min = Vec3D(value.bounds[0], value.bounds[1], value.bounds[2]);
    model.bounds.max = Vec3D(value.bounds[3], value.bounds[4], value.bounds[5]);
    model.header.collisionBox = model.bounds;
    // Assign allocations directly to the fresh model, so its original
    // destructor owns both even if the second allocation throws.
    model.vertices = new Vec3D[value.vertices];
    model.indices = new uint32[value.indices];
    for (std::uint32_t i = 0; i < value.vertices; ++i) {
        std::size_t at = 44 + std::size_t(i) * 12;
        Vec3D source(std::bit_cast<float>(word(value.bytes, at)),
            std::bit_cast<float>(word(value.bytes, at + 4)),
            std::bit_cast<float>(word(value.bytes, at + 8)));
        model.vertices[i] = fixCoordSystem(source);
    }
    for (std::uint32_t i = 0; i < value.indices; ++i)
        model.indices[i] = word(value.bytes, value.indices_offset + std::size_t(i) * 4);
}

// Authored capture/publication helpers follow native_adt_conversion.cpp. They
// are QA infrastructure; the native conversion kernel remains external.
class PrivateRun {
public:
    explicit PrivateRun(int stage_fd)
        : cwd_(::open(".", O_RDONLY | O_DIRECTORY | O_CLOEXEC)),
          saved_out_(::dup(STDOUT_FILENO)), saved_err_(::dup(STDERR_FILENO)),
          log_(::openat(stage_fd, "native.log", O_WRONLY | O_CREAT | O_EXCL
              | O_CLOEXEC | O_NOFOLLOW, 0600)) {
        if (cwd_.get() < 0 || saved_out_.get() < 0 || saved_err_.get() < 0 || log_.get() < 0)
            fail("qa_stage_capture");
        std::fflush(nullptr);
        if (::fchdir(stage_fd) != 0)
            fail("qa_stage_chdir");
        active_ = true;
        if (::dup2(log_.get(), STDOUT_FILENO) < 0 || ::dup2(log_.get(), STDERR_FILENO) < 0) {
            restore();
            fail("qa_stage_capture");
        }
    }
    ~PrivateRun() { restore(); }
    PrivateRun(PrivateRun const&) = delete;
    PrivateRun& operator=(PrivateRun const&) = delete;
    void finish() {
        std::fflush(nullptr);
        bool synced = ::fsync(log_.get()) == 0;
        restore();
        if (!synced)
            fail("qa_log_sync");
    }
private:
    void restore() noexcept {
        if (!active_)
            return;
        std::fflush(nullptr);
        bool ok = ::dup2(saved_out_.get(), STDOUT_FILENO) >= 0;
        ok = (::dup2(saved_err_.get(), STDERR_FILENO) >= 0) && ok;
        ok = (::fchdir(cwd_.get()) == 0) && ok;
        active_ = false;
        if (!ok)
            std::_Exit(5);
    }
    Fd cwd_, saved_out_, saved_err_, log_;
    bool active_ = false;
};

void read_at(int fd, std::span<std::uint8_t> bytes, std::size_t offset) {
    std::size_t done = 0;
    while (done < bytes.size()) {
        auto got = ::pread(fd, bytes.data() + done, bytes.size() - done,
            static_cast<off_t>(offset + done));
        if (got < 0 && errno == EINTR)
            continue;
        if (got <= 0)
            fail("qa_output_read");
        done += static_cast<std::size_t>(got);
    }
}

void check_output(int fd, Fixture const& value) {
    // Original fwrite/fclose results are unchecked. Inspect exact structural
    // extents without rewriting the output or duplicating its conversion.
    std::uint8_t prefix[88] {};
    read_at(fd, prefix, 0);
    if (!magic(prefix, std::string_view("VMAP04E\0", 8))
        || word(prefix, 8) != value.vertices || word(prefix, 12) != 1
        || word(prefix, 16) != 0 || word(prefix, 20) != std::uint32_t(ModelFlags::IsM2)
        || word(prefix, 24) != 0 || word(prefix, 28) != 0 || word(prefix, 56) != 0
        || !magic(View(prefix).subspan(60), "GRP ") || word(prefix, 64) != 8
        || word(prefix, 68) != 1 || word(prefix, 72) != value.indices
        || !magic(View(prefix).subspan(76), "INDX")
        || word(prefix, 80) != 4 + 2 * value.indices || word(prefix, 84) != value.indices)
        fail("qa_output_header");
    for (std::size_t i = 0; i < 6; ++i)
        if (word(prefix, 32 + i * 4) != word(value.bytes, 16 + i * 4))
            fail("qa_output_bounds");
    std::uint8_t vertex_header[12] {};
    read_at(fd, vertex_header, 88 + std::size_t(value.indices) * 4);
    if (!magic(vertex_header, "VERT") || word(vertex_header, 4) != 4 + 12 * value.vertices
        || word(vertex_header, 8) != value.vertices)
        fail("qa_output_header");
}

std::size_t convert(Fixture const& value, char const* destination) {
    // Lexical path handling uses the standard library. Never pass a caller's
    // output path to the original writer's fopen.
    std::filesystem::path output(destination);
    auto leaf = output.filename().string();
    if (leaf.empty() || leaf == "." || leaf == "..")
        fail("qa_output_path");
    auto parent_path = output.parent_path();
    if (parent_path.empty())
        parent_path = ".";
    Fd parent(::open(parent_path.c_str(), O_RDONLY | O_DIRECTORY | O_CLOEXEC | O_NOFOLLOW));
    if (parent.get() < 0)
        fail("qa_output_parent");
    struct stat existing {};
    if (::fstatat(parent.get(), leaf.c_str(), &existing, AT_SYMLINK_NOFOLLOW) == 0)
        fail("qa_output_exists");
    if (errno != ENOENT)
        fail("qa_output_metadata");

    std::string synthetic_name = "synthetic";
    Model model(synthetic_name);
    prepare(model, value);
    std::string stage_name;
    for (unsigned attempt = 0; attempt < 100; ++attempt) {
        stage_name = ".native-m2-stage-" + std::to_string(::getpid()) + "-" + std::to_string(attempt);
        if (::mkdirat(parent.get(), stage_name.c_str(), 0700) == 0)
            break;
        if (errno != EEXIST || attempt == 99)
            fail("qa_stage_create");
    }
    Fd stage(::openat(parent.get(), stage_name.c_str(), O_RDONLY | O_DIRECTORY | O_CLOEXEC | O_NOFOLLOW));
    struct stat stage_stat {};
    if (stage.get() < 0 || ::fstat(stage.get(), &stage_stat) != 0
        || !S_ISDIR(stage_stat.st_mode) || (stage_stat.st_mode & 0777) != 0700
        || stage_stat.st_uid != ::geteuid())
        fail("qa_stage_metadata");
    {
        PrivateRun captured(stage.get());
        if (!model.ConvertToVMAPModel("native.vmap"))
            fail("qa_native_convert");
        captured.finish();
    }

    Fd produced(::openat(stage.get(), "native.vmap", O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK));
    struct stat before {}, after {}, named {};
    if (produced.get() < 0 || ::fstat(produced.get(), &before) != 0
        || !S_ISREG(before.st_mode) || before.st_nlink != 1)
        fail("qa_stage_output");
    if (before.st_size < 0 || static_cast<std::uintmax_t>(before.st_size) != value.output_bytes
        || static_cast<std::uintmax_t>(before.st_size) > value.output_cap)
        fail("qa_output_extent");
    check_output(produced.get(), value);
    if (::fstat(produced.get(), &after) != 0
        || ::fstatat(stage.get(), "native.vmap", &named, AT_SYMLINK_NOFOLLOW) != 0
        || !unchanged(before, after) || !unchanged(before, named))
        fail("qa_output_changed");
    if (::fsync(produced.get()) != 0 || ::fsync(stage.get()) != 0)
        fail("qa_output_sync");
    if (::linkat(stage.get(), "native.vmap", parent.get(), leaf.c_str(), 0) != 0)
        fail(errno == EEXIST ? "qa_output_exists" : "qa_output_publish");
    if (::fsync(parent.get()) != 0)
        fail("qa_output_sync");
    return value.output_bytes;
}
} // namespace

int main(int argc, char** argv) {
    if (argc != 3) {
        std::puts("{\"status\":\"error\",\"code\":\"qa_usage\"}");
        return 2;
    }
    ::umask(0077);
    try {
        auto bytes = read_complete(argv[1]);
        auto value = fixture(bytes);
        auto output_bytes = convert(value, argv[2]);
        std::printf("{\"status\":\"ok\",\"bytes\":%zu,\"vertices\":%u,\"indices\":%u,"
            "\"original_converter\":true,\"original_parser\":false,\"network\":false}\n",
            output_bytes, value.vertices, value.indices);
        return 0;
    } catch (Failure const& error) {
        std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n", error.code);
        return 4;
    } catch (std::bad_alloc const&) {
        std::puts("{\"status\":\"error\",\"code\":\"qa_allocation_limit\"}");
        return 4;
    } catch (...) {
        std::puts("{\"status\":\"error\",\"code\":\"qa_internal\"}");
        return 5;
    }
}
