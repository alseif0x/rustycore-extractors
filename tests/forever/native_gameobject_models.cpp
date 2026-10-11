// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only original gameobject-model list exporter QA.
// Pin 02245dcd245e7433e524577656177723d3e4992e, original
// TileAssembler.cpp:402-467 and WorldModel_Raw::Read/GroupModel_Raw::Read.
// Original sources/notices are materialized externally and remain unchanged.
// No parser, bound merge, geometry or native support function is substituted.
//
// CLI: native_gameobject_models <temp-list> <raw-input-directory> <new-stage>
// The list is exact VMAP04E\0 + repeated {id:u32,nameLen:u32,nameBytes}.
// Order, duplicate IDs and duplicate names are retained. Unique raw files are
// copied in full to stage/raw, together with temp_gameobject_models. PUBLIC
// TileAssembler(raw,native,1).exportGameobjectModels() produces
// stage/native/GameObjectModels.dtree. Original diagnostics go to native.log.
//
// This is only a positive-fixture QA route, not a malformed-native safety gate.
// Every supplied raw must have been structurally validated independently and
// contain at least one finite vertex across its groups. Export's void return
// and silent skips cannot count as success here: generated rows must exactly
// match every input ID/name in order, with finite ordered bounds and exact EOF.
// The original computes bounds from ALL vertices in ALL groups, irrespective
// of triangle use or declared group bounds. This adapter does not calculate a
// replacement bounding box. Empty list is admitted, with an eight-byte output.
//
// QA path admission is stricter than a pure in-memory string codec: printable
// ASCII basenames1..499, no /, backslash, colon, literal '.'/'..' or reserved
// temp_gameobject_models. No filename normalization occurs. Individual list/raw
// input <=64MiB, <=100000 records, <=256MiB unique copied bytes AND cumulative
// raw bytes read by the exporter (duplicate references charged again). Output
// <=256MiB, log <=16MiB are post-write checks, not native allocator/write caps;
// the coordinator bounds the process externally. Source metadata identities
// and complete copied bytes are checked before/after the original invocation.
//
// New stage/raw/native directories0700; wrapper files reserved O_EXCL0600.
// The original output writer keeps its fopen("wb") body on our reserved inode;
// named/held inode checks and an exact singleton output inventory guard it.
// Temporary QA outputs are flushed and reread completely, without fsync or a
// crash-durability guarantee. Native file/allocator caps remain external.
// No convertWorld2, VMO conversion, CASC, DB2, dynamic runtime consumer or queries.
#include "TileAssembler.h"
#include <algorithm>
#include <array>
#include <bit>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <limits>
#include <map>
#include <memory>
#include <string>
#include <string_view>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#ifdef G3D_DEBUG
#error "gameobject QA requires original G3D release configuration"
#endif

namespace {
constexpr std::size_t MaxInput = 64 * 1024 * 1024;
constexpr std::size_t MaxTotal = 256 * 1024 * 1024;
constexpr std::size_t MaxRecords = 100000;
constexpr std::size_t MaxLog = 16 * 1024 * 1024;
using Bytes = std::vector<unsigned char>;
using File = std::unique_ptr<FILE, int (*)(FILE*)>;
struct Failure { char const* code; };

bool identity(struct stat const& a, struct stat const& b) {
    return a.st_dev == b.st_dev && a.st_ino == b.st_ino && a.st_mode == b.st_mode
        && a.st_uid == b.st_uid && a.st_gid == b.st_gid && a.st_nlink == b.st_nlink;
}
bool unchanged(struct stat const& a, struct stat const& b) {
    return identity(a, b) && a.st_size == b.st_size
        && a.st_mtim.tv_sec == b.st_mtim.tv_sec && a.st_mtim.tv_nsec == b.st_mtim.tv_nsec
        && a.st_ctim.tv_sec == b.st_ctim.tv_sec && a.st_ctim.tv_nsec == b.st_ctim.tv_nsec;
}
struct stat info(FILE* file) {
    struct stat state {};
    if (::fstat(::fileno(file), &state) || !S_ISREG(state.st_mode) || state.st_size < 0)
        throw Failure{"qa_file_type"};
    return state;
}
std::size_t size(struct stat const& state, std::size_t cap) {
    if (!S_ISREG(state.st_mode) || state.st_size < 0
        || static_cast<unsigned long long>(state.st_size) > cap)
        throw Failure{"qa_file_limit"};
    return static_cast<std::size_t>(state.st_size);
}
File open_file(std::filesystem::path const& path, bool create) {
    int flags = O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK;
    flags |= create ? O_RDWR | O_CREAT | O_EXCL : O_RDONLY;
    int fd = ::open(path.c_str(), flags, 0600);
    if (fd < 0) throw Failure{"qa_file_open"};
    FILE* file = ::fdopen(fd, create ? "w+b" : "rb");
    if (!file) { ::close(fd); throw Failure{"qa_file_open"}; }
    File result(file, &std::fclose);
    auto state = info(file);
    if (create && ((state.st_mode & 07777) != 0600 || state.st_nlink != 1
                   || state.st_uid != ::geteuid()))
        throw Failure{"qa_file_mode"};
    return result;
}
void read_exact(FILE* file, void* buffer, std::size_t count) {
    if (count && std::fread(buffer, 1, count, file) != count) throw Failure{"qa_read"};
}
void write_exact(FILE* file, void const* buffer, std::size_t count) {
    if (count && std::fwrite(buffer, 1, count, file) != count) throw Failure{"qa_write"};
}
void eof(FILE* file) {
    if (std::fgetc(file) != EOF || std::ferror(file)) throw Failure{"qa_tail"};
}
void sync_file(FILE* file) {
    if (std::fflush(file)) throw Failure{"qa_sync"};
}
void rewind_file(FILE* file) {
    if (std::fseek(file, 0, SEEK_SET)) throw Failure{"qa_seek"};
}
void add(std::size_t& total, std::size_t value, std::size_t cap) {
    if (value > cap - total) throw Failure{"qa_total_limit"};
    total += value;
}

struct Snapshot {
    std::filesystem::path path;
    struct stat before {};
    std::size_t bytes;
    explicit Snapshot(std::filesystem::path source) : path(std::move(source)), bytes(0) {
        if (::lstat(path.c_str(), &before)) throw Failure{"qa_input_open"};
        bytes = size(before, MaxInput);
    }
    File open() const {
        auto file = open_file(path, false);
        if (!unchanged(before, info(file.get()))) throw Failure{"qa_input_changed"};
        return file;
    }
    void verify() const {
        struct stat after {};
        if (::lstat(path.c_str(), &after) || !unchanged(before, after))
            throw Failure{"qa_input_changed"};
    }
    Bytes read() const {
        auto file = open();
        Bytes data(bytes);
        read_exact(file.get(), data.data(), data.size()); eof(file.get());
        if (!unchanged(before, info(file.get()))) throw Failure{"qa_input_changed"};
        verify();
        return data;
    }
};

void directory(std::filesystem::path const& path) {
    if (::mkdir(path.c_str(), 0700)) throw Failure{"qa_stage_exists"};
    struct stat state {};
    if (::lstat(path.c_str(), &state) || !S_ISDIR(state.st_mode)
        || (state.st_mode & 07777) != 0700 || state.st_uid != ::geteuid())
        throw Failure{"qa_directory_mode"};
}

void compare_copy(Snapshot const& source, std::filesystem::path const& copy) {
    auto input = source.open(), copied = open_file(copy, false);
    auto copy_before = info(copied.get());
    if ((copy_before.st_mode & 07777) != 0600 || copy_before.st_nlink != 1
        || copy_before.st_uid != ::geteuid()) throw Failure{"qa_file_mode"};
    if (size(copy_before, MaxInput) != source.bytes) throw Failure{"qa_copy_size"};
    std::array<unsigned char, 8192> a {}, b {};
    for (std::size_t remaining = source.bytes; remaining;) {
        auto count = std::min(remaining, a.size());
        read_exact(input.get(), a.data(), count); read_exact(copied.get(), b.data(), count);
        if (!std::equal(a.begin(), a.begin() + count, b.begin())) throw Failure{"qa_copy_bytes"};
        remaining -= count;
    }
    eof(input.get()); eof(copied.get());
    struct stat copy_named {};
    if (::lstat(copy.c_str(), &copy_named) || !unchanged(copy_before, copy_named)
        || !unchanged(copy_before, info(copied.get()))
        || !unchanged(source.before, info(input.get())))
        throw Failure{"qa_input_changed"};
    source.verify();
}

void copy_raw(Snapshot const& source, std::filesystem::path const& destination) {
    if (source.bytes < 24) throw Failure{"qa_raw_header"};
    auto input = source.open(), output = open_file(destination, true);
    std::array<unsigned char, 8192> buffer {};
    for (std::size_t remaining = source.bytes; remaining;) {
        auto count = std::min(remaining, buffer.size());
        read_exact(input.get(), buffer.data(), count);
        if (remaining == source.bytes
            && std::string_view(reinterpret_cast<char const*>(buffer.data()), 8)
               != std::string_view("VMAP04E\0", 8)) throw Failure{"qa_raw_magic"};
        write_exact(output.get(), buffer.data(), count);
        remaining -= count;
    }
    eof(input.get()); sync_file(output.get());
    if (!unchanged(source.before, info(input.get()))) throw Failure{"qa_input_changed"};
    source.verify();
    compare_copy(source, destination);
}

class NativeLog {
public:
    explicit NativeLog(std::filesystem::path const& path)
        : file_(open_file(path, true)), out_(::dup(STDOUT_FILENO)), err_(::dup(STDERR_FILENO)) {
        if (out_ < 0 || err_ < 0 || std::fflush(nullptr)) { close_saved(); throw Failure{"qa_log_setup"}; }
        if (::dup2(::fileno(file_.get()), STDOUT_FILENO) < 0) { close_saved(); throw Failure{"qa_log_setup"}; }
        if (::dup2(::fileno(file_.get()), STDERR_FILENO) < 0) {
            if (::dup2(out_, STDOUT_FILENO) < 0) std::_Exit(4);
            close_saved(); throw Failure{"qa_log_setup"};
        }
        active_ = true;
    }
    NativeLog(NativeLog const&) = delete;
    NativeLog& operator=(NativeLog const&) = delete;
    ~NativeLog() { if (active_) restore(); close_saved(); }
    void finish() {
        if (!restore()) throw Failure{"qa_log_flush"};
        sync_file(file_.get()); size(info(file_.get()), MaxLog);
    }
private:
    void close_saved() noexcept {
        if (out_ >= 0) { ::close(out_); out_ = -1; }
        if (err_ >= 0) { ::close(err_); err_ = -1; }
    }
    bool restore() noexcept {
        bool okay = std::fflush(nullptr) == 0;
        if (::dup2(out_, STDOUT_FILENO) < 0 || ::dup2(err_, STDERR_FILENO) < 0) std::_Exit(4);
        active_ = false;
        return okay;
    }
    File file_;
    int out_, err_;
    bool active_ = false;
};

uint32 word(Bytes const& data, std::size_t& at) {
    if (at > data.size() || data.size() - at < 4) throw Failure{"qa_list_extent"};
    auto result = uint32(data[at]) | (uint32(data[at + 1]) << 8)
        | (uint32(data[at + 2]) << 16) | (uint32(data[at + 3]) << 24);
    at += 4;
    return result;
}
struct Row { uint32 id; std::string_view name; };
std::vector<Row> parse_list(Bytes const& data) {
    if (data.size() < 8 || std::string_view(reinterpret_cast<char const*>(data.data()), 8)
        != std::string_view("VMAP04E\0", 8)) throw Failure{"qa_list_magic"};
    std::vector<Row> rows;
    std::size_t at = 8;
    while (at != data.size()) {
        if (rows.size() == MaxRecords) throw Failure{"qa_record_limit"};
        auto id = word(data, at), length = word(data, at);
        if (length == 0 || length >= 500 || length > data.size() - at)
            throw Failure{"qa_name_extent"};
        std::string_view name(reinterpret_cast<char const*>(data.data() + at), length);
        if (name == "." || name == ".." || name == "temp_gameobject_models") throw Failure{"qa_name"};
        for (unsigned char c : name)
            if (c < 32 || c > 126 || c == '/' || c == '\\' || c == ':') throw Failure{"qa_name"};
        rows.push_back({id, name});
        at += length;
    }
    return rows;
}

std::size_t verify_output(FILE* file, std::vector<Row> const& rows, std::size_t expected) {
    if (size(info(file), MaxTotal) != expected) throw Failure{"qa_output_size"};
    rewind_file(file);
    std::array<char, 8> magic {};
    read_exact(file, magic.data(), magic.size());
    if (std::string_view(magic.data(), magic.size()) != "VMAP_4.E") throw Failure{"qa_output_magic"};
    for (auto const& row : rows) {
        uint32 id, length;
        read_exact(file, &id, 4); read_exact(file, &length, 4);
        if (id != row.id || length != row.name.size()) throw Failure{"qa_output_row"};
        std::array<char, 500> name {};
        read_exact(file, name.data(), length);
        if (std::string_view(name.data(), length) != row.name) throw Failure{"qa_output_row"};
        std::array<float, 6> bounds {};
        read_exact(file, bounds.data(), sizeof(bounds));
        for (float value : bounds) if (!std::isfinite(value)) throw Failure{"qa_output_bounds"};
        for (std::size_t axis = 0; axis < 3; ++axis)
            if (bounds[axis] > bounds[axis + 3]) throw Failure{"qa_output_bounds"};
    }
    eof(file);
    return expected;
}
}

int main(int argc, char** argv) try {
    static_assert(std::endian::native == std::endian::little);
    static_assert(sizeof(float) == 4 && sizeof(uint32) == 4);
    static_assert(std::numeric_limits<float>::is_iec559);
    if (argc != 4) {
        std::puts("{\"status\":\"error\",\"code\":\"qa_usage\"}"); return 2;
    }
    ::umask(0077);
    Snapshot list(std::filesystem::absolute(argv[1]));
    auto list_bytes = list.read();
    auto rows = parse_list(list_bytes);
    auto raw_input = std::filesystem::absolute(argv[2]);
    struct stat raw_dir_before {};
    if (::lstat(raw_input.c_str(), &raw_dir_before) || !S_ISDIR(raw_dir_before.st_mode))
        throw Failure{"qa_raw_directory"};
    std::map<std::string_view, Snapshot> sources;
    std::size_t copied_bytes = 0, read_work = 0, expected_output = list.bytes;
    add(expected_output, rows.size() * 24, MaxTotal);
    for (auto const& row : rows) {
        auto found = sources.find(row.name);
        if (found == sources.end()) {
            Snapshot source(raw_input / std::string(row.name));
            if (source.bytes < 24) throw Failure{"qa_raw_header"};
            add(copied_bytes, source.bytes, MaxTotal);
            found = sources.emplace(row.name, std::move(source)).first;
        }
        add(read_work, found->second.bytes, MaxTotal);
    }
    auto stage = std::filesystem::absolute(argv[3]);
    directory(stage); directory(stage / "raw"); directory(stage / "native");
    auto copied_list = stage / "raw" / "temp_gameobject_models";
    {
        auto file = open_file(copied_list, true);
        write_exact(file.get(), list_bytes.data(), list_bytes.size()); sync_file(file.get());
    }
    compare_copy(list, copied_list);
    for (auto const& [name, source] : sources) copy_raw(source, stage / "raw" / std::string(name));
    auto output_path = stage / "native" / "GameObjectModels.dtree";
    auto output = open_file(output_path, true);
    auto reserved = info(output.get());
    {
        NativeLog log(stage / "native.log");
        VMAP::TileAssembler assembler((stage / "raw").string(), (stage / "native").string(), 1);
        assembler.exportGameobjectModels();
        log.finish();
    }
    auto actual = info(output.get());
    struct stat named {};
    if (::lstat(output_path.c_str(), &named) || !identity(reserved, actual) || !unchanged(actual, named))
        throw Failure{"qa_output_changed"};
    sync_file(output.get());
    auto output_bytes = verify_output(output.get(), rows, expected_output);
    std::size_t inventory = 0;
    for (auto const& entry : std::filesystem::directory_iterator(stage / "native")) {
        if (entry.path().filename() != "GameObjectModels.dtree") throw Failure{"qa_output_inventory"};
        ++inventory;
    }
    if (inventory != 1) throw Failure{"qa_output_inventory"};
    for (auto const& [name, source] : sources) compare_copy(source, stage / "raw" / std::string(name));
    compare_copy(list, copied_list);
    struct stat raw_dir_after {};
    if (::lstat(raw_input.c_str(), &raw_dir_after) || !unchanged(raw_dir_before, raw_dir_after))
        throw Failure{"qa_input_changed"};
    if (::lstat(output_path.c_str(), &named) || !unchanged(actual, named)
        || !unchanged(actual, info(output.get()))) throw Failure{"qa_output_changed"};
    std::printf("{\"status\":\"ok\",\"rows\":%zu,\"list_bytes\":%zu,\"raw_copies\":%zu,"
                "\"raw_copied_bytes\":%zu,\"raw_read_bytes\":%zu,\"bytes\":%zu,"
                "\"export_calls\":1,\"raw_read_calls\":%zu,\"complete_output\":true}\n",
                rows.size(), list.bytes, sources.size(), copied_bytes, read_work, output_bytes, rows.size());
    return 0;
} catch (Failure const& error) {
    std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n", error.code); return 4;
} catch (...) {
    std::puts("{\"status\":\"error\",\"code\":\"qa_failure\"}"); return 4;
}
