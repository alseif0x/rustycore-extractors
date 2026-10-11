// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic QA of original pinned DB2 direct-record access.
// Reference 02245dcd245e7433e524577656177723d3e4992e:
// DB2FileLoader.cpp, DB2Meta.cpp, ExtractorDB2LoadInfo.h:61-106 and
// gameobject_extract.cpp:88-158. All original sources/notices remain unchanged.
// This file implements only bounded fixture I/O, a DB2FileSource memory adapter,
// candidate formatting and JSON. It substitutes no loader, getter or metadata.
//
// CLI: native_gameobject_sources <complete-synthetic.db2> <new-output.json>
// PUBLIC original Load/GetRecordCount/GetRecord/GetId/GetUInt32("FileDataID")
// and GetRecordCopyCount/GetRecordCopy run over a fresh original loader.
// Original GameobjectDisplayInfoLoadInfo includes all 15 flattened entries:
// FileDataID is Fields[7], resolving to physical field 1/array element 0.
// No AutoProduceData/GetMaxId/copy replay, CASC, model extraction or runtime.
// JSON: {direct:[[id,fdid]],copies:[[new,source]],
//        candidates:[[id,fdid,"FILEXXXXXXXX.xxx"]]} retaining order/duplicates.
// Only candidate FDID zero is omitted. Candidate naming is QA source logic,
// not an execution of ExtractGameobjectModels or its model-success decisions.
//
// Trusted, independently validated synthetic positive fixtures ONLY. The
// envelope guard does not establish general malformed-native safety. Input
// <=16MiB; direct<=floor(1M/14), copies<=100k; output<=16MiB. Native allocations
// and execution are additionally limited by the parent process. Stable ordinary
// input is checked before/after. Output is exclusive0600 under a private0700
// parent, only created after complete native reading. Complete reread is checked;
// there is no fsync/crash-durability promise. Failed writes may leave a partial
// private file, never reported as success or overwritten. Native diagnostics
// are suppressed during the call; stdout contains only static codes/counts.
#include "DB2FileLoader.h"
#include "ExtractorDB2LoadInfo.h"
#include <algorithm>
#include <array>
#include <bit>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <limits>
#include <memory>
#include <string>
#include <utility>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

namespace {
constexpr std::size_t MaxBytes = 16 * 1024 * 1024;
constexpr uint32 MaxDirect = 1000000 / 14;
constexpr uint32 MaxCopies = 100000;
using Bytes = std::vector<unsigned char>;
using File = std::unique_ptr<FILE, int (*)(FILE*)>;
struct Failure { char const* code; };

bool same(struct stat const& a, struct stat const& b) {
    return a.st_dev == b.st_dev && a.st_ino == b.st_ino && a.st_mode == b.st_mode
        && a.st_uid == b.st_uid && a.st_gid == b.st_gid && a.st_nlink == b.st_nlink
        && a.st_size == b.st_size && a.st_mtim.tv_sec == b.st_mtim.tv_sec
        && a.st_mtim.tv_nsec == b.st_mtim.tv_nsec && a.st_ctim.tv_sec == b.st_ctim.tv_sec
        && a.st_ctim.tv_nsec == b.st_ctim.tv_nsec;
}
struct stat info(FILE* file) {
    struct stat s {};
    if (::fstat(::fileno(file), &s) || !S_ISREG(s.st_mode) || s.st_size < 0)
        throw Failure{"qa_file_type"};
    return s;
}
File open_file(std::filesystem::path const& path, bool create) {
    int flags = O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK;
    flags |= create ? O_RDWR | O_CREAT | O_EXCL : O_RDONLY;
    int fd = ::open(path.c_str(), flags, 0600);
    if (fd < 0) throw Failure{create ? "qa_output_exists_or_io" : "qa_input_io"};
    FILE* f = ::fdopen(fd, create ? "w+b" : "rb");
    if (!f) { ::close(fd); throw Failure{"qa_file_io"}; }
    File result(f, &std::fclose);
    auto s = info(f);
    if (create && ((s.st_mode & 07777) != 0600 || s.st_nlink != 1 || s.st_uid != ::geteuid()))
        throw Failure{"qa_output_mode"};
    return result;
}
void read_exact(FILE* f, void* data, std::size_t n) {
    if (n && std::fread(data, 1, n, f) != n) throw Failure{"qa_read"};
}
void eof(FILE* f) {
    if (std::fgetc(f) != EOF || std::ferror(f)) throw Failure{"qa_tail"};
}
struct Input {
    std::filesystem::path path;
    File file;
    struct stat before;
    Bytes bytes;
    explicit Input(std::filesystem::path p)
        : path(std::move(p)), file(open_file(path, false)), before(info(file.get())) {
        if (static_cast<unsigned long long>(before.st_size) > MaxBytes) throw Failure{"qa_input_limit"};
        bytes.resize(static_cast<std::size_t>(before.st_size));
        read_exact(file.get(), bytes.data(), bytes.size()); eof(file.get()); verify();
    }
    void verify() const {
        struct stat named {};
        if (::lstat(path.c_str(), &named) || !same(before, named) || !same(before, info(file.get())))
            throw Failure{"qa_input_changed"};
    }
};
uint32 word(Bytes const& b, std::size_t at) {
    if (at > b.size() || b.size() - at < 4) throw Failure{"qa_header"};
    return uint32(b[at]) | (uint32(b[at + 1]) << 8) | (uint32(b[at + 2]) << 16) | (uint32(b[at + 3]) << 24);
}
void envelope(Bytes const& b) {
    if (b.size() < 240 || std::memcmp(b.data(), "WDC5", 4) || word(b, 4) != 5
        || word(b, 140) != 9 || word(b, 176) != 9 || word(b, 156) != 0x16D1DD61
        || (b[172] & 1) || word(b, 184) != 0 || word(b, 200) > 1)
        throw Failure{"qa_profile"};
    uint32 count = word(b, 136), sections = word(b, 200);
    if (count > MaxDirect) throw Failure{"qa_record_limit"};
    if (!sections) {
        if (count || word(b, 148) || word(b, 188) || word(b, 192) || word(b, 196) || b.size() != 240)
            throw Failure{"qa_empty_profile"};
        return;
    }
    if (b.size() < 496 || word(b, 188) != 216 || !word(b, 144)
        || word(b, 204) || word(b, 208) || word(b, 216) != count
        || word(b, 220) != word(b, 148) || word(b, 232) || word(b, 236)
        || word(b, 228) != std::uint64_t(count) * 4 || word(b, 240) > MaxCopies)
        throw Failure{"qa_section_profile"};
    std::uint64_t metadata = 496ull + word(b, 192) + word(b, 196);
    std::uint64_t ids = metadata + std::uint64_t(word(b, 144)) * count + word(b, 148);
    std::uint64_t end = ids + std::uint64_t(count) * 4 + std::uint64_t(word(b, 240)) * 8;
    if (word(b, 212) != metadata || end != b.size()) throw Failure{"qa_extent"};
    // The checked-source profile does not reproduce the original unsigned
    // overflow in MinId+ordinal repair. Such fixtures belong to Rust negatives.
    for (uint32 i = 0; i < count; ++i)
        if (word(b, static_cast<std::size_t>(ids + i * 4ull)) <= word(b, 160)
            && std::uint64_t(word(b, 160)) + i > std::numeric_limits<uint32>::max())
            throw Failure{"qa_id_overflow"};
}

class MemorySource final : public DB2FileSource {
public:
    explicit MemorySource(Bytes const& bytes) : bytes_(bytes) { }
    bool IsOpen() const override { return true; }
    bool Read(void* out, std::size_t count) override {
        if (count > bytes_.size() - at_) return false;
        if (count) std::memcpy(out, bytes_.data() + at_, count);
        at_ += count; return true;
    }
    int64 GetPosition() const override { return static_cast<int64>(at_); }
    bool SetPosition(int64 position) override {
        if (position < 0 || static_cast<std::uint64_t>(position) > bytes_.size()) return false;
        at_ = static_cast<std::size_t>(position); return true;
    }
    int64 GetFileSize() const override { return static_cast<int64>(bytes_.size()); }
    char const* GetFileName() const override { return "synthetic-gameobject.db2"; }
    DB2EncryptedSectionHandling HandleEncryptedSection(DB2SectionHeader const&) const override {
        throw Failure{"qa_encrypted_section"};
    }
private:
    Bytes const& bytes_;
    std::size_t at_ = 0;
};
class Silence {
public:
    Silence() {
        if (std::fflush(nullptr)) throw Failure{"qa_log"};
        out_ = ::dup(STDOUT_FILENO); err_ = ::dup(STDERR_FILENO);
        int sink = ::open("/dev/null", O_WRONLY | O_CLOEXEC);
        if (out_ < 0 || err_ < 0 || sink < 0) {
            if (sink >= 0) ::close(sink);
            close_saved(); throw Failure{"qa_log"};
        }
        if (::dup2(sink, STDOUT_FILENO) < 0) { ::close(sink); close_saved(); throw Failure{"qa_log"}; }
        if (::dup2(sink, STDERR_FILENO) < 0) {
            ::close(sink);
            if (::dup2(out_, STDOUT_FILENO) < 0) std::_Exit(4);
            close_saved(); throw Failure{"qa_log"};
        }
        ::close(sink); active_ = true;
    }
    ~Silence() { if (active_) restore(); close_saved(); }
    Silence(Silence const&) = delete;
    Silence& operator=(Silence const&) = delete;
    void finish() { if (!restore()) throw Failure{"qa_log_flush"}; }
private:
    bool restore() noexcept {
        bool okay = std::fflush(nullptr) == 0;
        if (::dup2(out_, STDOUT_FILENO) < 0 || ::dup2(err_, STDERR_FILENO) < 0) std::_Exit(4);
        active_ = false; return okay;
    }
    void close_saved() noexcept {
        if (out_ >= 0) { ::close(out_); out_ = -1; }
        if (err_ >= 0) { ::close(err_); err_ = -1; }
    }
    int out_ = -1, err_ = -1;
    bool active_ = false;
};
struct Rows {
    std::vector<std::pair<uint32, uint32>> direct;
    std::vector<DB2RecordCopy> copies;
};
Rows native_read(Bytes const& data) {
    Silence silence;
    MemorySource source(data);
    DB2FileLoader loader;
    auto const& load = GameobjectDisplayInfoLoadInfo::Instance;
    if (load.GetFieldIndexByName("FileDataID") != std::make_pair(int32(1), int32(0)))
        throw Failure{"qa_load_info"};
    loader.Load(&source, &load);
    if (source.GetPosition() != source.GetFileSize()) throw Failure{"qa_native_extent"};
    Rows rows;
    uint32 count = loader.GetRecordCount(), copies = loader.GetRecordCopyCount();
    if (count != word(data, 136) || count > MaxDirect || copies > MaxCopies
        || copies != (word(data, 200) ? word(data, 240) : 0)) throw Failure{"qa_native_count"};
    rows.direct.reserve(count); rows.copies.reserve(copies);
    for (uint32 i = 0; i < count; ++i) {
        DB2Record record = loader.GetRecord(i);
        if (!record) throw Failure{"qa_native_missing_record"};
        rows.direct.emplace_back(record.GetId(), record.GetUInt32("FileDataID"));
    }
    for (uint32 i = 0; i < copies; ++i) rows.copies.push_back(loader.GetRecordCopy(i));
    silence.finish();
    return rows;
}
void append(std::string& s, std::string const& value) {
    if (value.size() > MaxBytes - s.size()) throw Failure{"qa_output_limit"};
    s += value;
}
std::string render(Rows const& rows, std::size_t& candidates) {
    std::string result = "{\"direct\":[";
    bool first = true;
    for (auto const& [id, fdid] : rows.direct) {
        if (!first) append(result, ",");
        first = false;
        append(result, "[" + std::to_string(id) + "," + std::to_string(fdid) + "]");
    }
    append(result, "],\"copies\":["); first = true;
    for (auto const& copy : rows.copies) {
        if (!first) append(result, ",");
        first = false;
        append(result, "[" + std::to_string(copy.NewRowId) + "," + std::to_string(copy.SourceRowId) + "]");
    }
    append(result, "],\"candidates\":["); first = true; candidates = 0;
    for (auto const& [id, fdid] : rows.direct) {
        if (!fdid) continue;
        if (!first) append(result, ",");
        first = false;
        std::array<char, 17> name {};
        if (std::snprintf(name.data(), name.size(), "FILE%08X.xxx", static_cast<unsigned>(fdid)) != 16)
            throw Failure{"qa_name"};
        append(result, "[" + std::to_string(id) + "," + std::to_string(fdid) + ",\"" + name.data() + "\"]");
        ++candidates;
    }
    append(result, "]}\n"); return result;
}
}

int main(int argc, char** argv) try {
    static_assert(std::endian::native == std::endian::little && sizeof(uint32) == 4 && sizeof(unsigned) == 4);
    if (argc != 3) { std::puts("{\"status\":\"error\",\"code\":\"qa_usage\"}"); return 2; }
    ::umask(0077);
    auto destination = std::filesystem::absolute(argv[2]);
    struct stat named {};
    if (::lstat(destination.c_str(), &named) == 0) throw Failure{"qa_output_exists"};
    if (errno != ENOENT) throw Failure{"qa_output_io"};
    if (::lstat(destination.parent_path().c_str(), &named) || !S_ISDIR(named.st_mode)
        || (named.st_mode & 07777) != 0700 || named.st_uid != ::geteuid())
        throw Failure{"qa_output_parent"};
    Input input(std::filesystem::absolute(argv[1]));
    envelope(input.bytes);
    Rows rows = native_read(input.bytes);
    std::size_t candidates = 0;
    auto json = render(rows, candidates);
    input.verify();
    auto output = open_file(destination, true);
    if (std::fwrite(json.data(), 1, json.size(), output.get()) != json.size()
        || std::fflush(output.get()) || std::fseek(output.get(), 0, SEEK_SET)) throw Failure{"qa_output_io"};
    auto written = info(output.get());
    if (static_cast<unsigned long long>(written.st_size) != json.size()) throw Failure{"qa_output_size"};
    std::array<char, 8192> buffer {};
    for (std::size_t at = 0; at < json.size();) {
        auto count = std::min(buffer.size(), json.size() - at);
        read_exact(output.get(), buffer.data(), count);
        if (std::memcmp(buffer.data(), json.data() + at, count)) throw Failure{"qa_output_bytes"};
        at += count;
    }
    eof(output.get()); input.verify();
    if (::lstat(destination.c_str(), &named) || !same(written, named) || !same(written, info(output.get())))
        throw Failure{"qa_output_changed"};
    if (std::fclose(output.release())) throw Failure{"qa_output_close"};
    std::printf("{\"status\":\"success\",\"direct_records\":%zu,\"copy_records\":%zu,\"candidate_records\":%zu,\"bytes\":%zu,\"load_calls\":1,\"get_record_calls\":%zu}\n",
                rows.direct.size(), rows.copies.size(), candidates, json.size(), rows.direct.size());
    return 0;
} catch (Failure const& e) {
    std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n", e.code); return 4;
} catch (...) {
    std::puts("{\"status\":\"error\",\"code\":\"qa_native_failure\"}"); return 4;
}
