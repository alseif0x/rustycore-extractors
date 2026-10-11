// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic QA adapter, not an extractor.
// Includes externally materialized, unmodified TrinityCore System.cpp at
// 02245dcd245e7433e524577656177723d3e4992e. Its original copyrights and
// GPL-2.0-or-later notice remain in that source; this repository does not import
// it. The build driver must hash every transitive source/header and retain the
// corresponding AUTHORS/COPYING and CascLib/fmt license notices.
//
// CLI: native_adt_conversion <adt.bin> <lookup.bin> <context.bin> <output.map>
// Lookup: RCELIQ01, two LE u32 counts (<=256 each), then type records
// {id:u32,sound:u8,material:u8}, material records {id:u32,flags:u32,lvf:i8}.
// Last duplicate ID wins. Exact EOF. Context is exactly 31 bytes: RCECONV1,
// map_id:u32, tile_y:u8, tile_x:u8, build:u32, pack:u8, input_cap:u32,
// chunk_cap:u32, output_cap:u32. This matches examples/convert_adt.rs.
//
// TRUSTED, STRUCTURALLY VALID SYNTHETIC FIXTURES ONLY. Original parseChunks
// scans payload bytes and is not a bounds-safe parser. Never feed malformed
// fuzz or client assets to this program. The adapter byte limits do not prove
// native rejection or safety. Its post-parse count covers retained interesting
// chunks, unlike Rust's aggregate framing count (which includes unknown tags).
//
// No original function is replaced; no CASC stub is linked. Only main is
// renamed. Compile this TU plus original loadlib.cpp and MapDefines.cpp with
// -std=c++20 -ffunction-sections -fdata-sections -Wl,--gc-sections,
// -fno-fast-math -ffp-contract=off. Include native_adt_compat first. The driver
// must inspect the retained symbol set: no CASC APIs or I/O, DB2 APIs,
// filesystem shim I/O or renamed original main may survive. Inert CascStorage,
// shared_ptr destruction and global initialization are expected. The shim
// permits dead CLI typechecking.
//
// Original stdio goes into a retained private stage log; output is written by
// original ConvertADT inside that fresh 0700 directory, checked and fsynced,
// then published by an exclusive hard link. Existing outputs are never opened
// for writing. No automatic removal of logs, stages or previous outputs.

#include <algorithm>
#include <bit>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <limits>
#include <span>
#include <string>
#include <string_view>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#ifndef RCE_PINNED_SYSTEM_CPP
#error "Define RCE_PINNED_SYSTEM_CPP as the verified external System.cpp path"
#endif
#define main rce_unused_original_main
#include RCE_PINNED_SYSTEM_CPP
#undef main

namespace {
using Bytes = std::vector<std::uint8_t>;
using View = std::span<std::uint8_t const>;
constexpr std::size_t InputLimit = 16 * 1024 * 1024;
constexpr std::size_t ChunkLimit = 4096;
constexpr std::size_t OutputLimit = 202112;
constexpr std::size_t LookupLimit = 16 + 256 * 6 + 256 * 9;
static_assert(std::endian::native == std::endian::little);
static_assert(sizeof(float) == 4 && std::numeric_limits<float>::is_iec559);
static_assert(sizeof(map_fileheader) == 44);

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

Bytes read_complete(char const* path, std::size_t cap) {
    Fd fd(::open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK));
    if (fd.get() < 0)
        fail("qa_input_open");
    struct stat before {}, after {}, named {};
    if (::fstat(fd.get(), &before) != 0 || !S_ISREG(before.st_mode))
        fail("qa_input_not_file");
    if (before.st_size < 0 || static_cast<std::uintmax_t>(before.st_size) > cap)
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

struct Context {
    std::uint32_t map_id;
    std::uint8_t tile_y;
    std::uint8_t tile_x;
    std::uint32_t build;
    bool pack;
    std::uint32_t input_cap;
    std::uint32_t chunk_cap;
    std::uint32_t output_cap;
};

Context context(View bytes) {
    if (bytes.size() != 31 || !magic(bytes, "RCECONV1") || bytes[18] > 1)
        fail("qa_context_format");
    Context value {word(bytes, 8), bytes[12], bytes[13], word(bytes, 14),
        bytes[18] != 0, word(bytes, 19), word(bytes, 23), word(bytes, 27)};
    if (value.tile_y >= 64 || value.tile_x >= 64
        || (value.build != 70170 && value.build != 70205)
        || value.input_cap > InputLimit || value.chunk_cap > ChunkLimit
        || value.output_cap > OutputLimit)
        fail("qa_context_policy");
    return value;
}

void install_lookup(View bytes) {
    if (bytes.size() < 16 || !magic(bytes, "RCELIQ01"))
        fail("qa_lookup_format");
    auto types = word(bytes, 8);
    auto materials = word(bytes, 12);
    if (types > 256 || materials > 256
        || bytes.size() != 16 + std::size_t(types) * 6 + std::size_t(materials) * 9)
        fail("qa_lookup_extent");
    LiquidTypes.clear();
    LiquidMaterials.clear();
    LiquidObjects.clear();
    std::size_t at = 16;
    for (std::uint32_t i = 0; i < types; ++i, at += 6)
        LiquidTypes[word(bytes, at)] = {bytes[at + 4], bytes[at + 5]};
    for (std::uint32_t i = 0; i < materials; ++i, at += 9) {
        LiquidMaterialEntry entry;
        entry.Flags = static_cast<LiquidMaterialFlags>(word(bytes, at + 4));
        entry.LVF = std::bit_cast<std::int8_t>(bytes[at + 8]);
        LiquidMaterials[word(bytes, at)] = entry;
    }
}

// This is an observation of the original parser's retained tree AFTER it has
// run, not a replacement parser or protection against malformed recursion.
template<class Chunks>
void count_retained(Chunks const& chunks, std::size_t cap, std::size_t& count) {
    for (auto const& [name, chunk] : chunks) {
        (void)name;
        if (count >= cap)
            fail("qa_retained_chunk_limit");
        ++count;
        count_retained(chunk.subchunks, cap, count);
    }
}

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

struct Result { std::size_t bytes; std::size_t retained_chunks; bool ignore_deep_water; };

Result convert(Bytes const& bytes, Context const& ctx, char const* destination) {
    // Lexical path handling uses the standard library, never the Boost shim.
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

    std::string stage_name;
    for (unsigned attempt = 0; attempt < 100; ++attempt) {
        stage_name = ".native-adt-stage-" + std::to_string(::getpid()) + "-" + std::to_string(attempt);
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

    std::size_t retained_chunks = 0;
    bool ignore = IsDeepWaterIgnored(ctx.map_id, ctx.tile_y, ctx.tile_x);
    {
        PrivateRun captured(stage.get());
        ChunkedFile adt;
        adt.data_size = static_cast<uint32>(bytes.size());
        adt.data = new uint8[bytes.size()];
        std::memcpy(adt.data, bytes.data(), bytes.size());
        adt.parseChunks();
        count_retained(adt.chunks, ctx.chunk_cap, retained_chunks);
        if (!adt.prepareLoadedData())
            fail("qa_native_prepare");
        CONF_allow_float_to_int = ctx.pack;
        if (!ConvertADT(adt, "synthetic", "native.map", ctx.tile_y, ctx.tile_x, ctx.build, ignore))
            fail("qa_native_convert");
        captured.finish();
    }

    Fd produced(::openat(stage.get(), "native.map", O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK));
    struct stat file_stat {};
    if (produced.get() < 0 || ::fstat(produced.get(), &file_stat) != 0
        || !S_ISREG(file_stat.st_mode) || file_stat.st_nlink != 1)
        fail("qa_stage_output");
    if (file_stat.st_size < 44 || static_cast<std::uintmax_t>(file_stat.st_size) > ctx.output_cap
        || static_cast<std::uintmax_t>(file_stat.st_size) > OutputLimit)
        fail("qa_output_limit");
    // Original fwrite return values are unchecked. Confirm its declared output
    // extent before publication without rewriting or repacking any bytes.
    std::uint8_t header[44] {};
    if (::pread(produced.get(), header, sizeof(header), 0) != static_cast<ssize_t>(sizeof(header))
        || !magic(header, "MAPS") || word(header, 4) != 10 || word(header, 8) != ctx.build)
        fail("qa_output_header");
    std::uint64_t end = sizeof(header);
    for (std::size_t at = 12; at < sizeof(header); at += 8) {
        auto offset = word(header, at);
        auto size = word(header, at + 4);
        if (offset == 0 && size == 0)
            continue;
        if (offset != end || size == 0)
            fail("qa_output_extent");
        end += size;
    }
    if (end != static_cast<std::uintmax_t>(file_stat.st_size))
        fail("qa_output_extent");
    if (::fsync(produced.get()) != 0 || ::fsync(stage.get()) != 0)
        fail("qa_output_sync");
    if (::linkat(stage.get(), "native.map", parent.get(), leaf.c_str(), 0) != 0)
        fail(errno == EEXIST ? "qa_output_exists" : "qa_output_publish");
    if (::fsync(parent.get()) != 0)
        fail("qa_output_sync");
    return {static_cast<std::size_t>(file_stat.st_size), retained_chunks, ignore};
}
} // namespace

int main(int argc, char** argv) {
    if (argc != 5) {
        std::puts("{\"status\":\"error\",\"code\":\"qa_usage\"}");
        return 2;
    }
    ::umask(0077);
    try {
        auto ctx = context(read_complete(argv[3], 31));
        install_lookup(read_complete(argv[2], LookupLimit));
        auto adt = read_complete(argv[1], ctx.input_cap);
        // Avoid even the original parser's subtract-eight underflow. This is
        // not a general structural check; the caller controls valid fixtures.
        if (adt.size() < 12)
            fail("qa_input_extent");
        auto result = convert(adt, ctx, argv[4]);
        std::printf("{\"status\":\"ok\",\"bytes\":%zu,\"retained_chunks\":%zu,"
            "\"ignore_deep_water\":%s,\"original_parser\":true,\"original_converter\":true,"
            "\"network\":false}\n", result.bytes, result.retained_chunks,
            result.ignore_deep_water ? "true" : "false");
        return 0;
    } catch (Failure const& error) {
        std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n", error.code);
        return 4;
    } catch (...) {
        std::puts("{\"status\":\"error\",\"code\":\"qa_internal\"}");
        return 5;
    }
}
