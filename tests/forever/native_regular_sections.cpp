// SPDX-License-Identifier: GPL-3.0-or-later
// Authored independent synthetic QA of original 02245dcd numeric WDC5 loading.
// CLI: native_regular_sections <complete-synthetic.db2> <new-output.json>
// Uses unchanged original DB2LoadInfo.h / SpellDurationLoadInfo::Instance,
// DB2FileLoader::Load, all three direct signed32 getters, AutoProduceData and
// AutoProduceRecordCopies. No replacement metadata/loader/materializer, DB,
// CASC, client bytes or runtime. Original notices/sources stay external.
// Only independently admitted complete positives, with 2..16 sections, small
// IDs and initialized eight-byte packed windows, may reach the original loader.
// Rust rejection/rollback is not compared with native malformed-input safety.
// Exact-width scalar bits, direct order, ordered copies and complete indexed
// materialization are emitted; pointer addresses/uninitialized slots are not.
// Source input is checked unchanged and output exclusive private0600 with full
// reread. Failed I/O may leave a private partial file; no durability is promised.
#include "DB2FileLoader.h"
#include "DB2LoadInfo.h"
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
constexpr uint32 MaxDirect = 4096;
constexpr uint32 MaxId = 4095;
constexpr uint32 MaxCopies = 4096;
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
struct Envelope { uint32 direct; uint32 copies; uint32 extent; };
Envelope envelope(Bytes const& b) {
    if (b.size() < 204 || std::memcmp(b.data(), "WDC5", 4) || word(b, 4) != 5
        || word(b, 140) != 3 || word(b, 176) != 3 || word(b, 156) != 0xA931BD2B
        || word(b, 148) || (b[172] & 1) || word(b, 184) || word(b, 188) != 72)
        throw Failure{"qa_profile"};
    uint32 count = word(b, 136), sections = word(b, 200), size = word(b, 144);
    uint32 minimum = word(b, 160), maximum = word(b, 164);
    if (count > MaxDirect || sections < 2 || sections > 16 || size < 12
        || minimum > maximum || maximum > MaxId) throw Failure{"qa_count_limit"};
    std::size_t columns = 216 + 40 * sections;
    if (columns > b.size() || b.size() - columns < 72) throw Failure{"qa_extent"};
    std::array<std::vector<uint32>, 3> palettes;
    std::size_t at = columns + 72;
    for (uint32 mode : {3u, 4u, 2u}) {
        std::size_t start = at;
        for (uint32 field = 0; field < 3; ++field) {
            auto c = columns + field * 24;
            uint32 compression = word(b, c + 8), extra = word(b, c + 4);
            if (compression > 5 || ((compression == 0 || compression == 1 || compression == 5) && extra))
                throw Failure{"qa_column"};
            if (compression != mode) continue;
            uint32 stride = mode == 2 ? 8 : 4;
            if (extra % stride || at > b.size() || extra > b.size() - at) throw Failure{"qa_column_extra"};
            if (mode != 2)
                for (uint32 j = 0; j < extra; j += 4) palettes[field].push_back(word(b, at + j));
            at += extra;
        }
        if ((mode == 4 && at - (columns + 72) != word(b, 196))
            || (mode == 2 && at - start != word(b, 192))) throw Failure{"qa_column_extent"};
    }
    uint32 total = 0, copies = 0, observedMax = 0;
    std::array<bool, MaxId + 1> ids {};
    for (uint32 section = 0; section < sections; ++section) {
        std::size_t sh = 204 + section * 40;
        uint32 n = word(b, sh + 12), nc = word(b, sh + 36);
        if (word(b, sh) || word(b, sh + 4) || word(b, sh + 8) != at
            || word(b, sh + 16) || word(b, sh + 20) || word(b, sh + 24) != n * 4ull
            || word(b, sh + 28) || word(b, sh + 32)
            || n > MaxDirect - total || nc > MaxCopies - copies)
            throw Failure{"qa_section"};
        std::uint64_t idsAt = std::uint64_t(at) + std::uint64_t(n) * size;
        std::uint64_t copyAt = idsAt + std::uint64_t(n) * 4;
        std::uint64_t end = copyAt + std::uint64_t(nc) * 8;
        if (end > b.size()) throw Failure{"qa_extent"};
        for (uint32 row = 0; row < n; ++row) {
            uint32 stored = word(b, static_cast<std::size_t>(idsAt + row * 4ull));
            std::uint64_t repaired = stored <= minimum ? std::uint64_t(minimum) + total + row : stored;
            if (repaired > maximum || ids[repaired]) throw Failure{"qa_ids"};
            ids[repaired] = true; observedMax = std::max(observedMax, uint32(repaired));
            for (uint32 field = 0; field < 3; ++field) {
                auto c = columns + field * 24;
                uint32 compression = word(b, c + 8);
                if (compression == 0) {
                    uint32 bitOffset = uint32(b[c]) | (uint32(b[c + 1]) << 8);
                    if (bitOffset % 8 || std::uint64_t(bitOffset / 8) + 4 > size) throw Failure{"qa_column"};
                } else if (compression != 2) {
                    uint32 bit = word(b, c + 12), width = word(b, c + 16);
                    std::uint64_t location = std::uint64_t(word(b, 180)) + bit / 8;
                    if (!width || width > 32 || location + 8 > size) throw Failure{"qa_packed_extent"};
                    std::uint64_t packed = 0;
                    std::memcpy(&packed, b.data() + at + std::size_t(row) * size + location, 8);
                    std::uint64_t value = (packed >> (bit % 8)) & ((std::uint64_t(1) << width) - 1);
                    if ((compression == 3 || compression == 4) && value >= palettes[field].size())
                        throw Failure{"qa_palette_index"};
                    if (compression == 4 && word(b, c + 20) != 1) throw Failure{"qa_palette_array"};
                }
            }
        }
        for (uint32 copy = 0; copy < nc; ++copy) {
            uint32 dest = word(b, static_cast<std::size_t>(copyAt + copy * 8ull));
            uint32 source = word(b, static_cast<std::size_t>(copyAt + copy * 8ull + 4));
            // Never execute native self-copy, even if its source would be skipped.
            if (dest > maximum || dest == source) throw Failure{"qa_copy"};
            observedMax = std::max(observedMax, dest);
        }
        total += n; copies += nc; at = static_cast<std::size_t>(end);
    }
    if (total != count || at != b.size()) throw Failure{"qa_extent"};
    return {total, copies, observedMax + 1};
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
    char const* GetFileName() const override { return "synthetic-regular-sections.db2"; }
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
using ScalarRow = std::array<uint32, 4>;
struct Rows {
    std::vector<ScalarRow> direct;
    std::vector<DB2RecordCopy> copies;
    std::vector<ScalarRow> materialized;
};
Rows native_read(Bytes const& data, Envelope const& expected) {
    Silence silence;
    MemorySource source(data);
    DB2FileLoader loader;
    auto const& load = SpellDurationLoadInfo::Instance;
    if (load.Meta->GetRecordSize() != 16 || load.FieldCount != 4)
        throw Failure{"qa_load_info"};
    loader.Load(&source, &load);
    if (source.GetPosition() != source.GetFileSize()) throw Failure{"qa_native_extent"};
    Rows rows;
    uint32 count = loader.GetRecordCount(), copies = loader.GetRecordCopyCount();
    if (count != expected.direct || copies != expected.copies) throw Failure{"qa_native_count"};
    rows.direct.reserve(count); rows.copies.reserve(copies);
    for (uint32 i = 0; i < count; ++i) {
        DB2Record record = loader.GetRecord(i);
        if (!record) throw Failure{"qa_native_missing_record"};
        ScalarRow row {record.GetId(), 0, 0, 0};
        for (uint32 field = 0; field < 3; ++field)
            row[field + 1] = std::bit_cast<uint32>(record.GetInt32(field, 0));
        rows.direct.push_back(row);
    }
    for (uint32 i = 0; i < copies; ++i) rows.copies.push_back(loader.GetRecordCopy(i));
    uint32 extent = 0;
    char** index = nullptr;
    std::unique_ptr<char[]> produced(loader.AutoProduceData(extent, index));
    std::unique_ptr<char*[]> ownedIndex(index);
    if (extent != expected.extent || extent > MaxId + 1) throw Failure{"qa_native_index_extent"};
    loader.AutoProduceRecordCopies(extent, index, produced.get());
    for (uint32 id = 0; id < extent; ++id) {
        if (!index[id]) continue;
        ScalarRow row {};
        std::memcpy(row.data(), index[id], 16);
        if (row[0] != id) throw Failure{"qa_native_materialized_id"};
        rows.materialized.push_back(row);
    }
    silence.finish();
    return rows;
}

void append(std::string& s, std::string const& value) {
    if (value.size() > MaxBytes - s.size()) throw Failure{"qa_output_limit"};
    s += value;
}
void render_row(std::string& result, ScalarRow const& row) {
    append(result, "[" + std::to_string(row[0]) + ",[" + std::to_string(row[1])
        + "," + std::to_string(row[2]) + "," + std::to_string(row[3]) + "]]");
}
std::string render(Rows const& rows) {
    std::string result = "{\"direct\":[";
    bool first = true;
    for (auto const& row : rows.direct) {
        if (!first) append(result, ",");
        first = false;
        render_row(result, row);
    }
    append(result, "],\"copies\":["); first = true;
    for (auto const& copy : rows.copies) {
        if (!first) append(result, ",");
        first = false;
        append(result, "[" + std::to_string(copy.NewRowId) + "," + std::to_string(copy.SourceRowId) + "]");
    }
    append(result, "],\"rows\":["); first = true;
    for (auto const& row : rows.materialized) {
        if (!first) append(result, ",");
        first = false;
        render_row(result, row);
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
    auto checked = envelope(input.bytes);
    Rows rows = native_read(input.bytes, checked);
    auto json = render(rows);
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
    std::printf("{\"status\":\"success\",\"direct_records\":%zu,\"copy_records\":%zu,\"rows\":%zu,\"bytes\":%zu,\"load_calls\":1,\"get_record_calls\":%zu,\"auto_produce_data_calls\":1,\"auto_produce_copy_calls\":1}\n",
                rows.direct.size(), rows.copies.size(), rows.materialized.size(), json.size(), rows.direct.size());
    return 0;
} catch (Failure const& e) {
    std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n", e.code); return 4;
} catch (...) {
    std::puts("{\"status\":\"error\",\"code\":\"qa_native_failure\"}"); return 4;
}
