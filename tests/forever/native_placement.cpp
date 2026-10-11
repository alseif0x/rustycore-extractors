// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic-only placement QA adapter.
// Executes externally materialized, unmodified TrinityCore source at
// 02245dcd245e7433e524577656177723d3e4992e: model.cpp Doodad::Extract,
// wmo.cpp MapObject::Extract, and ModelInstance.cpp ModelSpawn read/write.
// Original notices remain with the external source closure; no upstream
// implementation is imported here. The driver must record all source/header
// hashes, AUTHORS/COPYING and third-party notices, including G3D headers.
//
// CLI: native_placement <synthetic-fixture> <new-output>
// Fixture: magic RCEPLC01 (8), kind u8 (1 MDDF, 2 MODF), options u8
// (bit0 global WMO, bit1 exercise ADTOutputCache), reserved u16 zero,
// mapID/originalMapID/assignedID/nameLength/outputCap (five u32 LE).
// Header is 32 bytes; append opaque MDDF36 or MODF64 and nameLength bytes.
// Exact EOF, nonempty non-NUL ASCII names <=500, finite used floats, ordered WMO
// bounds. Global M2 rejects. These are QA admission policies, not assertions
// about native malformed-input safety. Input <=1024 and output <=562 bytes.
//
// Output is one original dir_bin record (38+nameLength for M2, 62+nameLength
// for WMO). IDs are supplied: the authored GenerateUniqueObjectId guard checks
// source unique ID, doodad ID zero and model class, permits exactly one call
// and returns assignedID. This does NOT execute the native ID allocator.
// Optional cache is compared with the emitted payload and parent bit cleared.
// The original ModelSpawn reader and writer must round-trip every output
// exactly. No authored placement arithmetic or serialization replaces them.
// Global WMO's original vector addition includes +0.0f on the third axis;
// compile without fast math/contraction so signed-zero behavior is observable.
//
// Compile original model.cpp, wmo.cpp and ModelInstance.cpp separately with
// function/data sections and linker GC; retain only the direct projectors and
// ModelSpawn codec. No CASC, Model::open, ExtractSingleModel, ExtractSet,
// BIH/geometry-query or G3D implementation should survive in the binary.
// Explicit aggregate initialization uses original inline AABox(Vector3,
// Vector3), avoiding the default constructor's G3D::fnan dependency. Do not
// add a replacement fnan or any permissive acquisition stub to make it link.
// Build ordinary release headers, C++20, -fno-fast-math -ffp-contract=off.
//
// Stdio is captured in a retained private 0700 stage; original routines only
// receive streams created there. Output is synced and linked exclusively.
// No client assets, acquisition, naming normalization or network is used.

#include <algorithm>
#include <array>
#include <bit>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <iostream>
#include <limits>
#include <new>
#include <span>
#include <string>
#include <type_traits>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include "adtfile.h"
#include "model.h"
#include "wmo.h"
#include "vmapexport.h"
#include "ModelInstance.h"

#if defined(VMAP_DEBUG) || defined(G3D_DEBUG)
#error "This QA closure requires ordinary release VMAP/G3D headers"
#endif

namespace {
using Bytes = std::vector<std::uint8_t>;
using View = std::span<std::uint8_t const>;
constexpr std::size_t InputLimit = 1024;
constexpr std::size_t OutputLimit = 562;
static_assert(std::endian::native == std::endian::little);
static_assert(sizeof(float) == 4 && std::numeric_limits<float>::is_iec559);
static_assert(sizeof(ADT::MDDF) == 36 && sizeof(ADT::MODF) == 64);
static_assert(sizeof(Vec3D) == 12 && sizeof(AaBox3D) == 24);
static_assert(sizeof(G3D::Vector3) == 12);
static_assert(std::is_trivially_copyable_v<ADT::MDDF>);
static_assert(std::is_trivially_copyable_v<ADT::MODF>);

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

bool unchanged(struct stat const& a, struct stat const& b) {
    return a.st_dev == b.st_dev && a.st_ino == b.st_ino
        && a.st_mode == b.st_mode && a.st_size == b.st_size
        && a.st_mtim.tv_sec == b.st_mtim.tv_sec
        && a.st_mtim.tv_nsec == b.st_mtim.tv_nsec
        && a.st_ctim.tv_sec == b.st_ctim.tv_sec
        && a.st_ctim.tv_nsec == b.st_ctim.tv_nsec;
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
    bool read_ok = true;
    while (offset < bytes.size()) {
        auto got = ::read(fd.get(), bytes.data() + offset, bytes.size() - offset);
        if (got < 0 && errno == EINTR)
            continue;
        if (got <= 0) { read_ok = false; break; }
        offset += static_cast<std::size_t>(got);
    }
    std::uint8_t tail = 0;
    ssize_t extra;
    do { extra = ::read(fd.get(), &tail, 1); } while (extra < 0 && errno == EINTR);
    if (::fstat(fd.get(), &after) != 0 || ::lstat(path, &named) != 0
        || !unchanged(before, after) || !unchanged(before, named))
        fail("qa_input_changed");
    if (!read_ok || extra != 0)
        fail("qa_input_read");
    return bytes;
}

std::uint32_t word(View bytes, std::size_t at) {
    if (at > bytes.size() || bytes.size() - at < 4)
        fail("qa_input_extent");
    return std::uint32_t(bytes[at]) | (std::uint32_t(bytes[at + 1]) << 8)
        | (std::uint32_t(bytes[at + 2]) << 16) | (std::uint32_t(bytes[at + 3]) << 24);
}

bool finite(std::uint32_t bits) { return (bits & 0x7f800000u) != 0x7f800000u; }

struct Fixture {
    bool wmo;
    bool global;
    bool cache;
    std::uint32_t map_id;
    std::uint32_t original_map_id;
    std::uint32_t assigned_id;
    std::uint32_t source_unique_id;
    std::uint32_t output_cap;
    View record;
    std::string name;
    std::size_t output_bytes;
};

Fixture fixture(View bytes) {
    constexpr std::array<std::uint8_t, 8> magic {'R','C','E','P','L','C','0','1'};
    if (bytes.size() < 32 || !std::equal(magic.begin(), magic.end(), bytes.begin())
        || (bytes[8] != 1 && bytes[8] != 2) || (bytes[9] & ~3u) != 0
        || bytes[10] != 0 || bytes[11] != 0)
        fail("qa_input_format");
    bool const wmo = bytes[8] == 2;
    if (!wmo && (bytes[9] & 1) != 0)
        fail("qa_input_context");
    std::size_t const record_size = wmo ? 64 : 36;
    auto name_length = word(bytes, 24);
    if (name_length == 0 || name_length > 500)
        fail("qa_input_name");
    if (bytes.size() != 32 + record_size + name_length)
        fail("qa_input_extent");
    View record = bytes.subspan(32, record_size);
    View name = bytes.subspan(32 + record_size);
    if (std::any_of(name.begin(), name.end(), [](auto byte) { return byte == 0 || byte > 127; }))
        fail("qa_input_name");
    // Position/rotation are consumed by both projectors; WMO also consumes
    // both source bounds. Ignore unrelated integer source flags/identifiers.
    for (std::size_t at = 8; at < (wmo ? 56u : 32u); at += 4)
        if (!finite(word(record, at)))
            fail("qa_input_nonfinite");
    if (wmo)
        for (std::size_t axis = 0; axis < 3; ++axis)
            if (std::bit_cast<float>(word(record, 32 + axis * 4))
                > std::bit_cast<float>(word(record, 44 + axis * 4)))
                fail("qa_input_bounds");
    auto output_cap = word(bytes, 28);
    std::size_t output_bytes = (wmo ? 62u : 38u) + name_length;
    if (output_cap > OutputLimit || output_bytes > output_cap)
        fail("qa_output_limit");
    return {wmo, (bytes[9] & 1) != 0, (bytes[9] & 2) != 0,
        word(bytes, 12), word(bytes, 16), word(bytes, 20), word(record, 4),
        output_cap, record,
        std::string(reinterpret_cast<char const*>(name.data()), name.size()), output_bytes};
}

struct IdGuard {
    bool active = false;
    bool wmo = false;
    std::uint32_t source = 0;
    std::uint32_t assigned = 0;
    unsigned calls = 0;
} id_guard;

class Stream {
public:
    Stream(int directory, char const* name, bool writing) {
        int flags = writing ? O_RDWR | O_CREAT | O_EXCL : O_RDONLY;
        int fd = ::openat(directory, name, flags | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK, 0600);
        if (fd < 0)
            fail("qa_stage_stream");
        value_ = ::fdopen(fd, writing ? "w+b" : "rb");
        if (!value_) { ::close(fd); fail("qa_stage_stream"); }
    }
    ~Stream() { if (value_) std::fclose(value_); }
    Stream(Stream const&) = delete;
    Stream& operator=(Stream const&) = delete;
    FILE* get() const { return value_; }
    void close_written() {
        bool ok = !std::ferror(value_) && std::fflush(value_) == 0;
        ok = (::fsync(::fileno(value_)) == 0) && ok;
        FILE* value = value_;
        value_ = nullptr;
        ok = (std::fclose(value) == 0) && ok;
        if (!ok) fail("qa_native_write");
    }
private:
    FILE* value_ = nullptr;
};

// Authored capture helpers follow native_m2_conversion.cpp. They do not
// replace any source placement math, serializer or reader.
class PrivateRun {
public:
    explicit PrivateRun(int stage)
        : cwd_(::open(".", O_RDONLY | O_DIRECTORY | O_CLOEXEC)),
          saved_out_(::dup(STDOUT_FILENO)), saved_err_(::dup(STDERR_FILENO)),
          log_(::openat(stage, "native.log", O_WRONLY | O_CREAT | O_EXCL
              | O_CLOEXEC | O_NOFOLLOW, 0600)) {
        if (cwd_.get() < 0 || saved_out_.get() < 0 || saved_err_.get() < 0 || log_.get() < 0)
            fail("qa_stage_capture");
        std::cout.flush();
        std::cerr.flush();
        std::fflush(nullptr);
        if (::fchdir(stage) != 0)
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
        std::cout.flush();
        std::cerr.flush();
        std::fflush(nullptr);
        bool synced = ::fsync(log_.get()) == 0;
        restore();
        if (!synced) fail("qa_log_sync");
    }
private:
    void restore() noexcept {
        if (!active_) return;
        std::cout.flush();
        std::cerr.flush();
        std::fflush(nullptr);
        bool ok = ::dup2(saved_out_.get(), STDOUT_FILENO) >= 0;
        ok = (::dup2(saved_err_.get(), STDERR_FILENO) >= 0) && ok;
        ok = (::fchdir(cwd_.get()) == 0) && ok;
        active_ = false;
        if (!ok) std::_Exit(5);
    }
    Fd cwd_, saved_out_, saved_err_, log_;
    bool active_ = false;
};

template<class T>
T opaque_record(View bytes) {
    static_assert(std::is_trivially_copyable_v<T>);
    if (bytes.size() != sizeof(T)) fail("qa_input_extent");
    T value;
    std::memcpy(static_cast<void*>(&value), bytes.data(), sizeof(value));
    return value;
}

Bytes staged_bytes(int stage, char const* name, std::size_t expected) {
    Fd fd(::openat(stage, name, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK));
    struct stat before {}, after {}, named {};
    if (fd.get() < 0 || ::fstat(fd.get(), &before) != 0 || !S_ISREG(before.st_mode)
        || before.st_nlink != 1 || before.st_size < 0
        || static_cast<std::uintmax_t>(before.st_size) != expected)
        fail("qa_output_extent");
    Bytes bytes(expected);
    std::size_t done = 0;
    while (done < bytes.size()) {
        auto got = ::read(fd.get(), bytes.data() + done, bytes.size() - done);
        if (got < 0 && errno == EINTR) continue;
        if (got <= 0) fail("qa_output_read");
        done += static_cast<std::size_t>(got);
    }
    if (::fstat(fd.get(), &after) != 0
        || ::fstatat(stage, name, &named, AT_SYMLINK_NOFOLLOW) != 0
        || !unchanged(before, after) || !unchanged(before, named))
        fail("qa_output_changed");
    return bytes;
}

void project(int stage, Fixture const& value) {
    PrivateRun captured(stage);
    Stream output(stage, "native.dirbin", true);
    std::vector<ADTOutputCache> cache;
    id_guard = {true, value.wmo, value.source_unique_id, value.assigned_id, 0};
    if (value.wmo) {
        auto record = opaque_record<ADT::MODF>(value.record);
        MapObject::Extract(record, value.name.c_str(), value.global, value.map_id,
            value.original_map_id, output.get(), value.cache ? &cache : nullptr);
    } else {
        auto record = opaque_record<ADT::MDDF>(value.record);
        Doodad::Extract(record, value.name.c_str(), value.map_id,
            value.original_map_id, output.get(), value.cache ? &cache : nullptr);
    }
    id_guard.active = false;
    if (id_guard.calls != 1) fail("qa_id_call_count");
    output.close_written();
    auto bytes = staged_bytes(stage, "native.dirbin", value.output_bytes);
    if (value.cache) {
        if (cache.size() != 1 || cache[0].Flags != (bytes[0] & ~2u)
            || cache[0].Data.size() != bytes.size() - 1
            || !std::equal(cache[0].Data.begin(), cache[0].Data.end(), bytes.begin() + 1))
            fail("qa_native_cache");
    } else if (!cache.empty()) {
        fail("qa_native_cache");
    }

    // Initialize every field via original inline constructors. A default
    // AABox would pull G3D::fnan from an additional implementation unit.
    G3D::Vector3 zero(0.0f, 0.0f, 0.0f);
    VMAP::ModelSpawn spawn {
        VMAP::ModelMinimalData {0, 0, 0, zero, 0.0f, G3D::AABox(zero, zero)},
        zero, std::string{}
    };
    Stream input(stage, "native.dirbin", false);
    if (!VMAP::ModelSpawn::readFromFile(input.get(), spawn)
        || std::fgetc(input.get()) != EOF || std::ferror(input.get()))
        fail("qa_native_reader");
    Stream roundtrip(stage, "roundtrip.dirbin", true);
    if (!VMAP::ModelSpawn::writeToFile(roundtrip.get(), spawn))
        fail("qa_native_writer");
    roundtrip.close_written();
    if (staged_bytes(stage, "roundtrip.dirbin", value.output_bytes) != bytes)
        fail("qa_native_roundtrip");
    captured.finish();
}

void convert(Fixture const& value, char const* destination) {
    std::filesystem::path output(destination);
    auto leaf = output.filename().string();
    if (leaf.empty() || leaf == "." || leaf == "..") fail("qa_output_path");
    auto parent_path = output.parent_path();
    if (parent_path.empty()) parent_path = ".";
    Fd parent(::open(parent_path.c_str(), O_RDONLY | O_DIRECTORY | O_CLOEXEC | O_NOFOLLOW));
    if (parent.get() < 0) fail("qa_output_parent");
    struct stat existing {};
    if (::fstatat(parent.get(), leaf.c_str(), &existing, AT_SYMLINK_NOFOLLOW) == 0)
        fail("qa_output_exists");
    if (errno != ENOENT) fail("qa_output_metadata");
    std::string stage_name;
    for (unsigned attempt = 0; attempt < 100; ++attempt) {
        stage_name = ".native-placement-stage-" + std::to_string(::getpid()) + "-" + std::to_string(attempt);
        if (::mkdirat(parent.get(), stage_name.c_str(), 0700) == 0) break;
        if (errno != EEXIST || attempt == 99) fail("qa_stage_create");
    }
    Fd stage(::openat(parent.get(), stage_name.c_str(), O_RDONLY | O_DIRECTORY | O_CLOEXEC | O_NOFOLLOW));
    struct stat metadata {};
    if (stage.get() < 0 || ::fstat(stage.get(), &metadata) != 0
        || !S_ISDIR(metadata.st_mode) || (metadata.st_mode & 0777) != 0700
        || metadata.st_uid != ::geteuid())
        fail("qa_stage_metadata");
    project(stage.get(), value);
    // Recheck the complete writer result after all native calls, before the
    // only publication operation. Native routines never receive this path.
    (void)staged_bytes(stage.get(), "native.dirbin", value.output_bytes);
    if (::fsync(stage.get()) != 0) fail("qa_output_sync");
    if (::linkat(stage.get(), "native.dirbin", parent.get(), leaf.c_str(), 0) != 0)
        fail(errno == EEXIST ? "qa_output_exists" : "qa_output_publish");
    if (::fsync(parent.get()) != 0) fail("qa_output_sync");
}
} // namespace

// Authored and deliberately narrow supplied-ID guard, not the source allocator.
uint32 GenerateUniqueObjectId(uint32 clientId, uint16 clientDoodadId, bool isWmo) {
    if (!id_guard.active || id_guard.calls != 0 || clientId != id_guard.source
        || clientDoodadId != 0 || isWmo != id_guard.wmo)
        fail("qa_id_guard");
    ++id_guard.calls;
    return id_guard.assigned;
}

int main(int argc, char** argv) {
    if (argc != 3) {
        std::puts("{\"status\":\"error\",\"code\":\"qa_usage\"}");
        return 2;
    }
    ::umask(0077);
    try {
        auto bytes = read_complete(argv[1]);
        auto value = fixture(bytes);
        convert(value, argv[2]);
        std::printf("{\"status\":\"ok\",\"bytes\":%zu,\"records\":1,"
            "\"projector_calls\":1,\"supplied_id_calls\":%u,\"reader_calls\":1,"
            "\"writer_calls\":1,\"cache_entries\":%u}\n",
            value.output_bytes, id_guard.calls, value.cache ? 1u : 0u);
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
