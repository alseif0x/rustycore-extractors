// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only caller of the unchanged pinned assembler.
// Reference: 02245dcd245e7433e524577656177723d3e4992e, TileAssembler.cpp,
// WorldModel.cpp, BoundingIntervalHierarchy.cpp and their original dependencies.
// Original sources/notices are supplied externally and are not imported here.
// No raw parser, geometry conversion, BIH algorithm or support method is replaced.
//
// CLI: native_vmo <m2|wmo> <complete-synthetic-raw-input> <new-private-stage>
// Input <=64 MiB is copied unchanged to raw/rawmodel. The PUBLIC original
// TileAssembler::convertRawFile("rawmodel") reads it and writes
// native/rawmodel.vmo. Original WorldModel::readFile/writeFile then writes
// roundtrip.vmo, which must be byte-identical. Each generated VMO is admitted
// only up to256 MiB; this post-write check is not a native allocation/write cap.
// The coordinator also bounds the process externally.
//
// ONLY structurally validated synthetic positive raw files belong here. The
// envelope check below is not a second raw parser and does not make unchecked
// native reads safe for malformed input. Original raw block-size/count quirks,
// zero-group output and zero-vertex group early returns remain untouched.
// Original WorldModel::readFile is permissive (including absent GMOD); complete
// write/read/write byte equality checks this generated output, not its general
// EOF validation or malformed-input safety. No queries, CASC, DB2, map assembly
// or runtime loading are exercised.
//
// Stage/subdirectories are new0700; fixed outputs/log are reserved exclusive0600.
// The original writers retain their fopen("wb") bodies; retained file descriptors
// and named inode checks confirm they wrote our private reserved files. Original
// stdout/stderr goes to native.log, never into the public structured diagnostic.
// Failed stages are retained for private inspection. Build with original G3D
// release configuration: its debug AABox assertion rejects empty-BIH NaNs.
#include "TileAssembler.h"
#include "WorldModel.h"
#include <algorithm>
#include <array>
#include <bit>
#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <limits>
#include <string>
#include <string_view>
#include <utility>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#ifdef G3D_DEBUG
#error "native_vmo empty BIH roundtrip requires original G3D release configuration"
#endif

namespace {
constexpr std::size_t MaxInputBytes = 64 * 1024 * 1024;
constexpr std::size_t MaxOutputBytes = 256 * 1024 * 1024;
constexpr std::size_t MaxLogBytes = 16 * 1024 * 1024;
using Bytes = std::vector<unsigned char>;
struct Failure { char const* code; };

class Descriptor {
public:
    explicit Descriptor(int value) : value_(value) {}
    Descriptor(Descriptor const&) = delete;
    Descriptor& operator=(Descriptor const&) = delete;
    Descriptor(Descriptor&& other) noexcept : value_(std::exchange(other.value_, -1)) {}
    ~Descriptor() { if (value_ >= 0) ::close(value_); }
    int get() const { return value_; }
private:
    int value_;
};

bool same_identity(struct stat const& a, struct stat const& b) {
    return a.st_dev == b.st_dev && a.st_ino == b.st_ino
        && a.st_mode == b.st_mode && a.st_uid == b.st_uid
        && a.st_gid == b.st_gid && a.st_nlink == b.st_nlink;
}

bool unchanged(struct stat const& a, struct stat const& b) {
    return same_identity(a, b) && a.st_size == b.st_size
        && a.st_mtim.tv_sec == b.st_mtim.tv_sec
        && a.st_mtim.tv_nsec == b.st_mtim.tv_nsec
        && a.st_ctim.tv_sec == b.st_ctim.tv_sec
        && a.st_ctim.tv_nsec == b.st_ctim.tv_nsec;
}

std::size_t regular_size(struct stat const& info, std::size_t maximum) {
    if (!S_ISREG(info.st_mode) || info.st_size < 0
        || static_cast<unsigned long long>(info.st_size) > maximum)
        throw Failure{"qa_extent"};
    return static_cast<std::size_t>(info.st_size);
}

void read_exact_at(int fd, unsigned char* output, std::size_t size, std::size_t offset) {
    while (size) {
        auto count = ::pread(fd, output, size, static_cast<off_t>(offset));
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) throw Failure{"qa_read"};
        auto taken = static_cast<std::size_t>(count);
        output += taken;
        offset += taken;
        size -= taken;
    }
}

void exact_eof(int fd, std::size_t size) {
    unsigned char tail {};
    ssize_t count;
    do { count = ::pread(fd, &tail, 1, static_cast<off_t>(size)); }
    while (count < 0 && errno == EINTR);
    if (count != 0) throw Failure{"qa_tail"};
}

struct Input {
    std::filesystem::path path;
    Descriptor fd;
    struct stat before {};
    Bytes bytes;

    explicit Input(std::filesystem::path value)
        : path(std::move(value)),
          fd(::open(path.c_str(), O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK)) {
        if (fd.get() < 0 || ::fstat(fd.get(), &before)) throw Failure{"qa_input_open"};
        auto size = regular_size(before, MaxInputBytes);
        if (size < 24) throw Failure{"qa_input_extent"};
        bytes.resize(size);
        read_exact_at(fd.get(), bytes.data(), size, 0);
        exact_eof(fd.get(), size);
        verify();
    }

    void verify() const {
        struct stat after {}, named {};
        if (::fstat(fd.get(), &after) || ::lstat(path.c_str(), &named)
            || !unchanged(before, after) || !unchanged(before, named))
            throw Failure{"qa_input_changed"};
    }
};

uint32 word(Bytes const& bytes, std::size_t at) {
    return uint32(bytes.at(at)) | (uint32(bytes.at(at + 1)) << 8)
        | (uint32(bytes.at(at + 2)) << 16) | (uint32(bytes.at(at + 3)) << 24);
}

void new_directory(std::filesystem::path const& path) {
    if (::mkdir(path.c_str(), 0700)) throw Failure{"qa_stage_exists"};
    struct stat info {};
    if (::lstat(path.c_str(), &info) || !S_ISDIR(info.st_mode)
        || (info.st_mode & 07777) != 0700 || info.st_uid != ::geteuid())
        throw Failure{"qa_stage_mode"};
}

class ReservedFile {
public:
    explicit ReservedFile(std::filesystem::path path)
        : path_(std::move(path)),
          fd_(::open(path_.c_str(), O_RDWR | O_CREAT | O_EXCL | O_CLOEXEC | O_NOFOLLOW, 0600)) {
        if (fd_.get() < 0 || ::fstat(fd_.get(), &identity_)) throw Failure{"qa_output_open"};
        if (!S_ISREG(identity_.st_mode) || (identity_.st_mode & 07777) != 0600
            || identity_.st_nlink != 1 || identity_.st_uid != ::geteuid()
            || identity_.st_size != 0)
            throw Failure{"qa_output_mode"};
    }

    int fd() const { return fd_.get(); }
    std::filesystem::path const& path() const { return path_; }

    std::size_t verify(std::size_t maximum) const {
        struct stat actual {}, named {};
        if (::fstat(fd(), &actual) || ::lstat(path_.c_str(), &named)
            || !same_identity(identity_, actual) || !unchanged(actual, named))
            throw Failure{"qa_output_changed"};
        auto size = regular_size(actual, maximum);
        exact_eof(fd(), size);
        return size;
    }

    std::size_t sync(std::size_t maximum) const {
        if (::fsync(fd())) throw Failure{"qa_output_sync"};
        return verify(maximum);
    }

    void write(Bytes const& bytes) const {
        std::size_t at = 0;
        while (at < bytes.size()) {
            auto count = ::write(fd(), bytes.data() + at, bytes.size() - at);
            if (count < 0 && errno == EINTR) continue;
            if (count <= 0) throw Failure{"qa_output_write"};
            at += static_cast<std::size_t>(count);
        }
        if (sync(MaxInputBytes) != bytes.size()) throw Failure{"qa_output_extent"};
    }
private:
    std::filesystem::path path_;
    Descriptor fd_;
    struct stat identity_ {};
};

// The only diagnostics during original calls are retained in the private stage.
// Restoration failure exits silently, preventing a public message being sent to
// an unintended descriptor. No upstream logging method is replaced.
class NativeLog {
public:
    explicit NativeLog(ReservedFile const& file)
        : file_(file), out_(::dup(STDOUT_FILENO)), err_(::dup(STDERR_FILENO)) {
        if (out_.get() < 0 || err_.get() < 0 || std::fflush(nullptr))
            throw Failure{"qa_log_setup"};
        if (::dup2(file_.fd(), STDOUT_FILENO) < 0) throw Failure{"qa_log_setup"};
        if (::dup2(file_.fd(), STDERR_FILENO) < 0) {
            if (::dup2(out_.get(), STDOUT_FILENO) < 0) std::_Exit(4);
            throw Failure{"qa_log_setup"};
        }
        active_ = true;
    }
    NativeLog(NativeLog const&) = delete;
    NativeLog& operator=(NativeLog const&) = delete;
    ~NativeLog() { if (active_) restore(); }

    void finish() {
        bool flushed = restore();
        if (!flushed) throw Failure{"qa_log_flush"};
        file_.sync(MaxLogBytes);
    }
private:
    bool restore() noexcept {
        bool flushed = std::fflush(nullptr) == 0;
        if (::dup2(out_.get(), STDOUT_FILENO) < 0 || ::dup2(err_.get(), STDERR_FILENO) < 0)
            std::_Exit(4);
        active_ = false;
        return flushed;
    }
    ReservedFile const& file_;
    Descriptor out_, err_;
    bool active_ = false;
};

void equal_files(ReservedFile const& first, ReservedFile const& second,
                 std::size_t expected) {
    if (first.verify(MaxOutputBytes) != expected || second.verify(MaxOutputBytes) != expected)
        throw Failure{"qa_roundtrip_extent"};
    std::array<unsigned char, 8192> left {}, right {};
    for (std::size_t at = 0; at < expected;) {
        auto count = std::min(expected - at, left.size());
        read_exact_at(first.fd(), left.data(), count, at);
        read_exact_at(second.fd(), right.data(), count, at);
        if (!std::equal(left.begin(), left.begin() + count, right.begin()))
            throw Failure{"qa_roundtrip_bytes"};
        at += count;
    }
    if (first.verify(MaxOutputBytes) != expected || second.verify(MaxOutputBytes) != expected)
        throw Failure{"qa_roundtrip_extent"};
}
}

int main(int argc, char** argv) try {
    static_assert(std::endian::native == std::endian::little);
    static_assert(sizeof(float) == 4 && sizeof(uint32) == 4);
    static_assert(std::numeric_limits<float>::is_iec559);
    if (argc != 4 || (std::string_view(argv[1]) != "m2" && std::string_view(argv[1]) != "wmo")) {
        std::puts("{\"status\":\"error\",\"code\":\"qa_usage\"}");
        return 2;
    }
    ::umask(0077);
    bool const is_m2 = std::string_view(argv[1]) == "m2";
    Input input(std::filesystem::absolute(argv[2]));
    if (std::string_view(reinterpret_cast<char const*>(input.bytes.data()), 8)
        != std::string_view("VMAP04E\0", 8))
        throw Failure{"qa_magic"};
    if (word(input.bytes, 20) != (is_m2 ? 1u : 0u)) throw Failure{"qa_class"};
    auto source_groups = word(input.bytes, 12);

    auto stage = std::filesystem::absolute(argv[3]);
    new_directory(stage);
    auto raw_dir = stage / "raw";
    auto native_dir = stage / "native";
    new_directory(raw_dir);
    new_directory(native_dir);
    ReservedFile raw(raw_dir / "rawmodel");
    raw.write(input.bytes);
    ReservedFile native(native_dir / "rawmodel.vmo");
    ReservedFile roundtrip(stage / "roundtrip.vmo");
    ReservedFile log(stage / "native.log");

    std::size_t bytes = 0, groups = 0, vertices = 0, triangles = 0, liquids = 0;
    {
        NativeLog capture(log);
        VMAP::TileAssembler assembler(raw_dir.string(), native_dir.string(), 1);
        if (!assembler.convertRawFile("rawmodel")) throw Failure{"qa_native_convert"};
        bytes = native.sync(MaxOutputBytes);
        VMAP::WorldModel restored;
        if (!restored.readFile(native.path().string())) throw Failure{"qa_native_read"};
        groups = restored.getGroupModels().size();
        if (groups != source_groups || restored.IsM2() != is_m2) throw Failure{"qa_native_counts"};
        for (auto const& group : restored.getGroupModels()) {
            vertices += group.GetVertices().size();
            triangles += group.GetTriangles().size();
            liquids += group.GetLiquid() != nullptr;
        }
        if (!restored.writeFile(roundtrip.path().string())) throw Failure{"qa_native_write"};
        if (roundtrip.sync(MaxOutputBytes) != bytes) throw Failure{"qa_roundtrip_extent"};
        capture.finish();
    }
    equal_files(native, roundtrip, bytes);
    if (raw.verify(MaxInputBytes) != input.bytes.size()) throw Failure{"qa_raw_changed"};
    // Keep source and private raw copy stable across all original calls. This
    // comparison is transport-only; it does not decode or reinterpret raw data.
    std::array<unsigned char, 8192> copied {};
    for (std::size_t at = 0; at < input.bytes.size();) {
        auto count = std::min(input.bytes.size() - at, copied.size());
        read_exact_at(raw.fd(), copied.data(), count, at);
        if (!std::equal(copied.begin(), copied.begin() + count, input.bytes.begin() + at))
            throw Failure{"qa_raw_changed"};
        at += count;
    }
    input.verify();
    std::printf("{\"status\":\"ok\",\"groups\":%zu,\"vertices\":%zu,\"triangles\":%zu,"
                "\"liquids\":%zu,\"bytes\":%zu,\"convert_calls\":1,\"raw_read_calls\":1,"
                "\"model_write_calls\":2,\"model_read_calls\":1,\"roundtrip_exact\":true}\n",
                groups, vertices, triangles, liquids, bytes);
    return 0;
} catch (Failure const& error) {
    // Only adapter-owned static codes are printed. Native exception text can
    // contain paths and is deliberately mapped to the generic failure below.
    std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n", error.code);
    return 4;
} catch (...) {
    std::puts("{\"status\":\"error\",\"code\":\"qa_failure\"}");
    return 4;
}
