// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: joint original collision assembly synthetic QA.
// Derived from the authored native_collision_world.cpp harness on this date.
// Pin 02245dcd245e7433e524577656177723d3e4992e. Original producer, Map/liquid
// loaders, ParsMapFiles, WDTFile/ADTFile, model converters, projectors, cache,
// IDs and dtree exporter are unchanged external sources with original notices.
// Only acquisition uses authored immutable complete memory-inventory adapters.
// No native CASC/name-first routing or runtime query equivalence is claimed.
// CLI: native_collision_assembled <inventory.bin> <new-stage>
// ABI: RCCASM01, u32 fileCount, u32 reserved(0), u32 flags(1); then entries:
// u32 FDID, u32 byteLen, u32 aliasLen, ASCII alias bytes, complete source bytes.
// All four DB2 tables are required. No trailing bytes. 256 files, 64MiB/file,
// 256MiB aggregate, 16MiB/DB2. Positive, independently prevalidated fixtures only.
// Native parsers are not hardened; malformed cases must remain Rust-only.
// Uses original ParsMapFiles with Threads=1; single selected doodad per WMO
// permits exact order comparison. Native preload interleaves global acquisition.
// raw/ includes final models, temp_gameobject_models and dir_bin/<map> files;
// startup/ archives first-phase raw bytes, native/ holds GameObjectModels.dtree.
// A fresh TileAssembler executes original convertWorld2 into vmaps/. All raw
// inputs remain open and are checked for identity and complete bytes afterward.
// Exact final inventory/bytes are compared independently by the parent driver.
// Private exclusive QA stage, full input/cache/list/inode guards, no publication
// transaction or crash durability claim. Parent supplies limits/network denial.
#include "CascHandles.h"
#include "adtfile.h"
#include "cascfile.h"
#include "ExtractorDB2LoadInfo.h"
#include "TileAssembler.h"
#include "vmapexport.h"
#include "wmo.h"
#include <CascLib.h>
#include <boost/filesystem/path.hpp>
#include <algorithm>
#include <array>
#include <bit>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <limits>
#include <memory>
#include <span>
#include <string>
#include <string_view>
#include <unordered_map>
#include <utility>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#ifdef G3D_DEBUG
#error "startup QA requires original G3D release configuration"
#endif

extern std::shared_ptr<CASC::Storage> CascStorage;
extern std::unordered_map<std::string, ExtractedModelData> ExtractedModels;
extern bool preciseVectorData;
void ReadMapTable();
void ParsMapFiles();
extern uint32 Threads;
void ReadLiquidMaterialTable();
void ReadLiquidTypeTable();

namespace {
using Bytes = std::vector<unsigned char>;
using View = std::span<unsigned char const>;
using File = std::unique_ptr<FILE, int (*)(FILE*)>;
namespace fs = std::filesystem;
constexpr std::size_t MaxFile = 64 * 1024 * 1024;
constexpr std::size_t MaxTotal = 256 * 1024 * 1024;
constexpr std::size_t MaxEnvelope = MaxTotal + 256 * (12 + 499 + 8) + 20;
constexpr std::size_t MaxOutput = 512 * 1024 * 1024;
constexpr std::size_t MaxOpens = 1000000;
struct Failure { char const* code; };
[[noreturn]] void fail(char const* code) { throw Failure{code}; }
void add(std::size_t& total, std::size_t value, std::size_t cap) {
    if (total > cap || value > cap - total) fail("qa_budget");
    total += value;
}
bool same(struct stat const& a, struct stat const& b) {
    return a.st_dev == b.st_dev && a.st_ino == b.st_ino && a.st_mode == b.st_mode
        && a.st_nlink == b.st_nlink && a.st_uid == b.st_uid && a.st_gid == b.st_gid
        && a.st_size == b.st_size && a.st_mtim.tv_sec == b.st_mtim.tv_sec
        && a.st_mtim.tv_nsec == b.st_mtim.tv_nsec && a.st_ctim.tv_sec == b.st_ctim.tv_sec
        && a.st_ctim.tv_nsec == b.st_ctim.tv_nsec;
}
struct stat info(FILE* f) {
    struct stat s{};
    if (::fstat(::fileno(f), &s) || !S_ISREG(s.st_mode) || s.st_size < 0) fail("qa_file_type");
    return s;
}
File open_file(fs::path const& path, bool create) {
    int fd = ::open(path.c_str(), O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK
        | (create ? O_RDWR | O_CREAT | O_EXCL : O_RDONLY), 0600);
    if (fd < 0) fail(create ? "qa_output_exists_or_io" : "qa_input_io");
    FILE* f = ::fdopen(fd, create ? "w+b" : "rb");
    if (!f) { ::close(fd); fail("qa_file_io"); }
    File file(f, &std::fclose);
    auto s = info(f);
    if (create && ((s.st_mode & 07777) != 0600 || s.st_nlink != 1 || s.st_uid != ::geteuid()))
        fail("qa_output_mode");
    return file;
}
void read_exact(FILE* f, void* to, std::size_t n) {
    if (n && std::fread(to, 1, n, f) != n) fail("qa_read");
}
void eof(FILE* f) { if (std::fgetc(f) != EOF || std::ferror(f)) fail("qa_tail"); }
struct Snapshot {
    fs::path path;
    File file;
    struct stat before;
    explicit Snapshot(fs::path p, std::size_t cap)
        : path(std::move(p)), file(open_file(path, false)), before(info(file.get())) {
        if (static_cast<unsigned long long>(before.st_size) > cap) fail("qa_file_limit");
        verify();
    }
    void verify() const {
        struct stat named{};
        if (::lstat(path.c_str(), &named) || !same(before, named) || !same(before, info(file.get())))
            fail("qa_input_changed");
    }
    Bytes read() const {
        if (std::fseek(file.get(), 0, SEEK_SET)) fail("qa_seek");
        Bytes bytes(static_cast<std::size_t>(before.st_size));
        read_exact(file.get(), bytes.data(), bytes.size()); eof(file.get()); verify();
        return bytes;
    }
    void matches(View bytes) const {
        if (bytes.size() != static_cast<std::size_t>(before.st_size)) fail("qa_size_changed");
        if (std::fseek(file.get(), 0, SEEK_SET)) fail("qa_seek");
        std::array<unsigned char, 8192> buffer{};
        for (std::size_t at = 0; at < bytes.size();) {
            auto n = std::min(buffer.size(), bytes.size() - at);
            read_exact(file.get(), buffer.data(), n);
            if (std::memcmp(buffer.data(), bytes.data() + at, n)) fail("qa_bytes_changed");
            at += n;
        }
        eof(file.get()); verify();
    }
};
void directory(fs::path const& p) {
    if (::mkdir(p.c_str(), 0700)) fail("qa_stage_exists");
    struct stat s{};
    if (::lstat(p.c_str(), &s) || !S_ISDIR(s.st_mode) || (s.st_mode & 07777) != 0700
        || s.st_uid != ::geteuid()) fail("qa_directory_mode");
}
void write_new(fs::path const& p, View bytes) {
    auto file = open_file(p, true);
    if ((!bytes.empty() && std::fwrite(bytes.data(), 1, bytes.size(), file.get()) != bytes.size())
        || std::fflush(file.get())) fail("qa_write");
    Snapshot copied(p, MaxFile);
    copied.matches(bytes);
    if (!same(info(file.get()), copied.before)) fail("qa_output_changed");
    if (std::fclose(file.release())) fail("qa_close");
}
std::string canonical(uint32 id) {
    std::array<char, 17> name{};
    if (std::snprintf(name.data(), name.size(), "FILE%08X.xxx", static_cast<unsigned>(id)) != 16)
        fail("qa_name");
    return name.data();
}
struct Cursor {
    View bytes; std::size_t at = 0;
    View take(std::size_t n) {
        if (at > bytes.size() || n > bytes.size() - at) fail("qa_inventory_extent");
        auto result = bytes.subspan(at, n); at += n; return result;
    }
    uint32 word() {
        auto b = take(4);
        return uint32(b[0]) | uint32(b[1]) << 8 | uint32(b[2]) << 16 | uint32(b[3]) << 24;
    }
};
struct Entry { uint32 id; std::string name, alias; View bytes; };
struct Later { uint32 kind, id; };
struct Inventory {
    std::vector<Entry> files;
    std::vector<Later> later;
    bool liquids = false;
    std::size_t opens = 0, named_opens = 0, id_opens = 0, read_bytes = 0;
    Entry const& by_id(uint32 id) const {
        auto it = std::find_if(files.begin(), files.end(), [id](auto const& e) { return e.id == id; });
        if (it == files.end()) fail("qa_missing_inventory_file");
        return *it;
    }
    Entry const& by_name(char const* name) const {
        auto it = std::find_if(files.begin(), files.end(), [name](auto const& e) {
            return e.name == name || (!e.alias.empty() && e.alias == name);
        });
        if (it == files.end()) fail("qa_missing_inventory_name");
        return *it;
    }
};
Inventory* inventory = nullptr;
void alias_check(std::string const& alias) {
    if (alias.empty()) return;
    if (alias.size() > 499 || std::any_of(alias.begin(), alias.end(), [](unsigned char c) {
        return c < 32 || c > 126 || c == ':';
    })) fail("qa_alias");
    auto pos = alias.find_last_of("/\\");
    auto plain = std::string_view(alias).substr(pos == std::string::npos ? 0 : pos + 1);
    if (plain.size() < 4) fail("qa_alias");
    // Use the actual helper only to reject a reserved output collision. The
    // acquisition alias itself remains exact and unmodified.
    std::string normalized(plain);
    NormalizeFileName(normalized);
    if (normalized == "temp_gameobject_models" || normalized == "native.log") fail("qa_alias");
}
Inventory parse_inventory(View bytes) {
    Cursor cursor{bytes};
    auto magic = cursor.take(8);
    if (std::memcmp(magic.data(), "RCCASM01", 8)) fail("qa_inventory_magic");
    auto files = cursor.word(), later = cursor.word(), flags = cursor.word();
    if (files > 256 || later != 0 || flags != 1) fail("qa_inventory_limits");
    Inventory result; result.liquids = (flags & 1) != 0;
    std::size_t total = 0;
    for (uint32 i = 0; i < files; ++i) {
        auto id = cursor.word(), length = cursor.word(), name_length = cursor.word();
        if (!id || id == std::numeric_limits<uint32>::max() || length > MaxFile || name_length > 499)
            fail("qa_inventory_entry");
        add(total, length, MaxTotal);
        auto name = cursor.take(name_length);
        std::string alias(reinterpret_cast<char const*>(name.data()), name.size()); alias_check(alias);
        Entry entry{id, canonical(id), std::move(alias), cursor.take(length)};
        for (auto const& prior : result.files)
            if (prior.id == id || prior.name == entry.alias || prior.alias == entry.name
                || (!entry.alias.empty() && entry.alias == prior.alias)) fail("qa_inventory_alias_conflict");
        result.files.push_back(std::move(entry));
    }
    for (uint32 i = 0; i < later; ++i) {
        Later request{cursor.word(), cursor.word()};
        if (request.kind > 1) fail("qa_later_kind");
        result.by_id(request.id); result.later.push_back(request);
    }
    if (cursor.at != bytes.size()) fail("qa_inventory_tail");
    if (result.by_id(GameobjectDisplayInfoLoadInfo::Instance.Meta->FileDataId).bytes.size() > 16 * 1024 * 1024)
        fail("qa_db2_limit");
    if (result.liquids)
        for (uint32 id : {MapLoadInfo::Instance.Meta->FileDataId, LiquidMaterialLoadInfo::Instance.Meta->FileDataId, LiquidTypeLoadInfo::Instance.Meta->FileDataId})
            if (result.by_id(id).bytes.size() > 16 * 1024 * 1024) fail("qa_db2_limit");
    return result;
}
struct Handle { Entry const* entry; std::size_t at = 0; };
Handle& handle(HANDLE value) { if (!value || !inventory) fail("qa_handle"); return *static_cast<Handle*>(value); }
void opened(bool named) {
    if (!inventory) fail("qa_not_initialized");
    add(inventory->opens, 1, MaxOpens);
    ++(named ? inventory->named_opens : inventory->id_opens);
}
class Capture {
    File file_;
    int out_ = -1, err_ = -1;
    bool active_ = false;
    void restore() noexcept {
        bool okay = std::fflush(nullptr) == 0;
        if (::dup2(out_, STDOUT_FILENO) < 0 || ::dup2(err_, STDERR_FILENO) < 0 || !okay) std::_Exit(4);
        active_ = false;
    }
public:
    explicit Capture(fs::path const& path) : file_(open_file(path, true)) {
        if (std::fflush(nullptr)) fail("qa_log");
        out_ = ::dup(STDOUT_FILENO); err_ = ::dup(STDERR_FILENO);
        if (out_ < 0 || err_ < 0) fail("qa_log");
        if (::dup2(::fileno(file_.get()), STDOUT_FILENO) < 0) fail("qa_log");
        if (::dup2(::fileno(file_.get()), STDERR_FILENO) < 0) {
            if (::dup2(out_, STDOUT_FILENO) < 0) std::_Exit(4);
            fail("qa_log");
        }
        active_ = true;
    }
    ~Capture() { if (active_) restore(); if (out_ >= 0) ::close(out_); if (err_ >= 0) ::close(err_); }
    void finish() { restore(); if (info(file_.get()).st_size > 16 * 1024 * 1024) fail("qa_log_limit"); }
};
struct Artifact { std::string name; Bytes bytes; };
std::vector<Artifact> snapshot_raw(fs::path const& raw) {
    std::vector<Artifact> result;
    std::size_t total = 0;
    for (auto const& entry : fs::directory_iterator(raw)) {
        auto name = entry.path().filename().string();
        if (name == "dir_bin") continue;
        Snapshot source(entry.path(), MaxFile);
        if ((source.before.st_mode & 07777) != 0600 || source.before.st_nlink != 1
            || source.before.st_uid != ::geteuid()) fail("qa_raw_mode");
        if (result.size() >= 513) fail("qa_raw_count");
        if (name != "temp_gameobject_models") {
            auto cached = ExtractedModels.find(name);
            if (cached == ExtractedModels.end() || !cached->second.HasCollision()) fail("qa_raw_cache_binding");
        }
        add(total, static_cast<std::size_t>(source.before.st_size), MaxOutput);
        auto bytes = source.read();
        if (bytes.size() < 8 || std::memcmp(bytes.data(), "VMAP04E\0", 8)) fail("qa_raw_magic");
        result.push_back({std::move(name), std::move(bytes)});
    }
    std::sort(result.begin(), result.end(), [](auto const& a, auto const& b) { return a.name < b.name; });
    for (auto const& [name, cached] : ExtractedModels) {
        auto state = cached.State.load(std::memory_order::relaxed);
        if (state == ExtractedModelData::InProgress) fail("qa_incomplete_cache");
        bool exists = std::any_of(result.begin(), result.end(), [&](auto const& f) { return f.name == name; });
        if (exists != (state == ExtractedModelData::Ok)) fail("qa_raw_cache_binding");
    }
    return result;
}
std::size_t check_dtree(View list, View dtree) {
    Cursor a{list}, b{dtree};
    if (std::memcmp(a.take(8).data(), "VMAP04E\0", 8)
        || std::memcmp(b.take(8).data(), "VMAP_4.E", 8)) fail("qa_list_magic");
    std::size_t count = 0;
    while (a.at != list.size()) {
        auto id = a.word(), len = a.word();
        if (len == 0 || len >= 500 || ++count > 71428) fail("qa_list_limit");
        auto name = a.take(len);
        if (b.word() != id || b.word() != len) fail("qa_dtree_rows");
        auto other = b.take(len);
        if (!std::equal(name.begin(), name.end(), other.begin())) fail("qa_dtree_rows");
        std::array<float, 6> bounds{};
        for (auto& value : bounds) {
            auto bits = b.word();
            if ((bits & 0x7f800000u) == 0x7f800000u) fail("qa_dtree_nonfinite");
            value = std::bit_cast<float>(bits);
        }
        for (std::size_t axis = 0; axis < 3; ++axis)
            if (bounds[axis] > bounds[axis + 3]) fail("qa_dtree_bounds");
    }
    if (b.at != dtree.size()) fail("qa_dtree_tail");
    return count;
}
} // namespace

// Authored fixture acquisition, deliberately not linked to CascHandles.cpp or
// CascLib implementation. The original ABI/private member layout is unchanged.
namespace CASC {
Storage::Storage(HANDLE value) : _handle(value) {}
Storage::~Storage() { if (_handle != inventory) std::abort(); }
Storage* Storage::Open(boost::filesystem::path const&, uint32, char const* product) {
    if (!inventory || !product || std::strcmp(product, "synthetic-world")) fail("qa_storage_profile");
    return new Storage(inventory);
}
File* Storage::OpenFile(char const* name, uint32, bool, bool) const {
    if (_handle != inventory || !name) fail("qa_storage_handle");
    auto const& entry = inventory->by_name(name); opened(true);
    return new File(new Handle{&entry});
}
File* Storage::OpenFile(uint32 id, uint32, bool, bool) const {
    if (_handle != inventory) fail("qa_storage_handle");
    auto const& entry = inventory->by_id(id); opened(false);
    return new File(new Handle{&entry});
}
bool Storage::HasTactKey(uint64) const { fail("qa_encrypted_fixture"); }
File::File(HANDLE value) : _handle(value) {}
File::~File() { delete static_cast<Handle*>(_handle); }
uint32 File::GetId() const { return handle(_handle).entry->id; }
int64 File::GetSize() const { return static_cast<int64>(handle(_handle).entry->bytes.size()); }
int64 File::GetPointer() const { return static_cast<int64>(handle(_handle).at); }
bool File::SetPointer(int64 position) {
    auto& h = handle(_handle);
    if (position < 0 || static_cast<std::uint64_t>(position) > h.entry->bytes.size()) fail("qa_source_seek");
    h.at = static_cast<std::size_t>(position); return true;
}
bool File::ReadFile(void* buffer, uint32 bytes, uint32* bytes_read) {
    auto& h = handle(_handle);
    if (bytes > h.entry->bytes.size() - h.at) fail("qa_source_extent");
    add(inventory->read_bytes, bytes, 2 * MaxOutput);
    if (bytes) std::memcpy(buffer, h.entry->bytes.data() + h.at, bytes);
    h.at += bytes; if (bytes_read) *bytes_read = bytes; return true;
}
char const* HumanReadableCASCError(uint32) { return "synthetic_fixture_io"; }
}
DWORD GetCascError() { return ERROR_SUCCESS; }

CASCFile::CASCFile(std::shared_ptr<CASC::Storage const> storage, char const* name, bool)
    : eof(false), buffer(nullptr), pointer(0), size(0) {
    std::unique_ptr<CASC::File> file(storage->OpenFile(name, CASC_LOCALE_ALL_WOW)); init(file.get(), name);
}
CASCFile::CASCFile(std::shared_ptr<CASC::Storage const> storage, uint32 id, std::string const& name, bool)
    : eof(false), buffer(nullptr), pointer(0), size(0) {
    std::unique_ptr<CASC::File> file(storage->OpenFile(id, CASC_LOCALE_ALL_WOW)); init(file.get(), name.c_str());
}
void CASCFile::init(CASC::File* file, char const*) {
    if (!file || file->GetSize() < 0 || static_cast<std::uint64_t>(file->GetSize()) > MaxFile) fail("qa_source_limit");
    size = static_cast<std::size_t>(file->GetSize());
    buffer = new char[size];
    uint32 got = 0;
    if (!file->ReadFile(buffer, static_cast<uint32>(size), &got) || got != size) fail("qa_source_read");
}
size_t CASCFile::read(void* destination, size_t bytes) {
    if (pointer > size || bytes > size - pointer) fail("qa_source_extent");
    if (bytes) std::memcpy(destination, buffer + pointer, bytes);
    pointer += bytes; return bytes;
}
void CASCFile::seek(int position) {
    if (position < 0 || static_cast<std::size_t>(position) > size) fail("qa_source_seek");
    pointer = static_cast<std::size_t>(position); eof = pointer >= size;
}
void CASCFile::seekRelative(int offset) {
    auto position = static_cast<std::int64_t>(pointer) + offset;
    if (position < 0 || static_cast<std::uint64_t>(position) > size) fail("qa_source_seek");
    pointer = static_cast<std::size_t>(position); eof = pointer >= size;
}
void CASCFile::close() { delete[] buffer; buffer = nullptr; eof = true; }

int main(int argc, char** argv) try {
    static_assert(std::endian::native == std::endian::little && sizeof(uint32) == 4 && sizeof(unsigned) == 4);
    static_assert(sizeof(float) == 4 && std::numeric_limits<float>::is_iec559);
    if (argc != 3) { std::puts("{\"status\":\"error\",\"code\":\"qa_usage\"}"); return 2; }
    ::umask(0077);
    Snapshot input(fs::absolute(argv[1]), MaxEnvelope);
    Bytes data = input.read();
    Inventory fixture = parse_inventory(data);
    inventory = &fixture;
    auto stage = fs::absolute(argv[2]);
    directory(stage); directory(stage / "raw"); directory(stage / "raw/dir_bin"); directory(stage / "startup"); directory(stage / "native");
    std::string work = (stage / "raw").string();
    szWorkDirWmo = work.c_str(); preciseVectorData = false; Threads = 1;
    if (!ExtractedModels.empty()) fail("qa_initial_cache");
    CascStorage.reset(CASC::Storage::Open(boost::filesystem::path(), 0, "synthetic-world"));
    std::size_t listed = 0, startup_models = 0, startup_opens = 0;
    std::size_t dir_files = 0, dir_bytes = 0, final_files = 0, final_bytes = 0;
    {
        Capture log(stage / "native.log");
        ExtractGameobjectModels();
        auto before = snapshot_raw(stage / "raw");
        auto list = std::find_if(before.begin(), before.end(), [](auto const& file) { return file.name == "temp_gameobject_models"; });
        if (list == before.end()) fail("qa_missing_temp_list");
        startup_models = before.size() - 1;
        startup_opens = fixture.opens;
        for (auto const& file : before) write_new(stage / "startup" / file.name, file.bytes);
        auto reserved = open_file(stage / "native/GameObjectModels.dtree", true);
        auto inode = info(reserved.get());
        VMAP::TileAssembler assembler((stage / "raw").string(), (stage / "native").string(), 1);
        assembler.exportGameobjectModels();
        Snapshot dtree(stage / "native/GameObjectModels.dtree", MaxFile);
        auto output = dtree.read();
        if (inode.st_dev != dtree.before.st_dev || inode.st_ino != dtree.before.st_ino
            || !same(info(reserved.get()), dtree.before)) fail("qa_dtree_inode");
        listed = check_dtree(list->bytes, output);
        ReadMapTable(); ReadLiquidMaterialTable(); ReadLiquidTypeTable();
        ParsMapFiles();
        for (auto const& file : before) {
            Snapshot unchanged(stage / "raw" / file.name, MaxFile); unchanged.matches(file.bytes);
            Snapshot archived(stage / "startup" / file.name, MaxFile); archived.matches(file.bytes);
        }
        auto world_raw = snapshot_raw(stage / "raw");
        std::vector<Snapshot> guarded_inputs;
        std::vector<Bytes> guarded_bytes;
        for (auto const& file : world_raw) {
            guarded_inputs.emplace_back(stage / "raw" / file.name, MaxFile);
            guarded_bytes.push_back(file.bytes);
        }
        for (auto const& entry : fs::directory_iterator(stage / "raw/dir_bin")) {
            std::string name = entry.path().filename().string();
            if (name.size() < 4 || name.size() > 10 || !std::all_of(name.begin(), name.end(), [](char c) { return c >= '0' && c <= '9'; }))
                fail("qa_dir_bin_name");
            Snapshot dir(entry.path(), MaxFile); auto bytes = dir.read();
            if ((dir.before.st_mode & 07777) != 0600 || dir.before.st_nlink != 1 || dir.before.st_uid != ::geteuid()) fail("qa_dir_bin_mode");
            add(dir_files, 1, 4096); add(dir_bytes, bytes.size(), MaxOutput);
            guarded_inputs.push_back(std::move(dir));
            guarded_bytes.push_back(std::move(bytes));
        }
        directory(stage / "vmaps");
        VMAP::TileAssembler final_assembler((stage / "raw").string(), (stage / "vmaps").string(), 1);
        if (!final_assembler.convertWorld2()) fail("qa_convert_world");
        for (std::size_t i = 0; i < guarded_inputs.size(); ++i)
            guarded_inputs[i].matches(guarded_bytes[i]);
        if (snapshot_raw(stage / "raw").size() != world_raw.size()
            || std::distance(fs::directory_iterator(stage / "raw/dir_bin"), fs::directory_iterator()) != static_cast<std::ptrdiff_t>(dir_files))
            fail("qa_raw_inventory_changed");
        std::size_t final_directories = 0;
        for (auto const& entry : fs::recursive_directory_iterator(stage / "vmaps")) {
            struct stat named{};
            if (::lstat(entry.path().c_str(), &named)) fail("qa_final_stat");
            if (S_ISDIR(named.st_mode)) {
                if ((named.st_mode & 07777) != 0700 || named.st_uid != ::geteuid()) fail("qa_final_directory");
                add(final_directories, 1, 4096);
                continue;
            }
            Snapshot artifact(entry.path(), 128 * 1024 * 1024);
            if ((artifact.before.st_mode & 07777) != 0600 || artifact.before.st_nlink != 1
                || artifact.before.st_uid != ::geteuid()) fail("qa_final_mode");
            auto bytes = artifact.read();
            if (bytes.size() < 8 || std::memcmp(bytes.data(), "VMAP_4.E", 8)) fail("qa_final_magic");
            add(final_files, 1, 65536); add(final_bytes, bytes.size(), MaxOutput);
        }
        Snapshot final_dtree(stage / "vmaps/GameObjectModels.dtree", MaxFile);
        final_dtree.matches(output);
        dtree.matches(output);
        if (std::distance(fs::directory_iterator(stage / "native"), fs::directory_iterator()) != 1)
            fail("qa_native_inventory");
        input.matches(data);
        log.finish();
    }
    std::size_t collision = 0, doodad_only = 0, failed = 0;
    for (auto const& [name, model] : ExtractedModels) {
        (void)name;
        switch (model.State.load(std::memory_order::relaxed)) {
            case ExtractedModelData::Ok: ++collision; break;
            case ExtractedModelData::OkNoCollision: ++doodad_only; break;
            case ExtractedModelData::Failed: ++failed; break;
            default: fail("qa_incomplete_cache");
        }
    }
    CascStorage.reset();
    inventory = nullptr;
    std::printf("{\"status\":\"success\",\"startup_rows\":%zu,\"startup_raw_files\":%zu,\"startup_source_opens\":%zu,\"source_opens\":%zu,\"named_source_opens\":%zu,\"id_source_opens\":%zu,\"source_read_bytes\":%zu,\"cache_collision\":%zu,\"cache_doodad_only\":%zu,\"cache_failed\":%zu,\"dir_files\":%zu,\"dir_bytes\":%zu,\"map_and_liquids_loaded\":true,\"original_ParsMapFiles\":true,\"threads\":1,\"original_convertWorld2\":true,\"final_files\":%zu,\"final_bytes\":%zu}\n",
        listed, startup_models, startup_opens, fixture.opens, fixture.named_opens, fixture.id_opens,
        fixture.read_bytes, collision, doodad_only, failed, dir_files, dir_bytes, final_files, final_bytes);
    return 0;
} catch (Failure const& e) {
    // On failure the fixture object may already be destroyed. Terminate without
    // consulting it from static shared_ptr destruction; no success is emitted.
    inventory = nullptr;
    std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n", e.code);
    std::fflush(stdout); std::_Exit(4);
} catch (...) {
    inventory = nullptr;
    std::puts("{\"status\":\"error\",\"code\":\"qa_native_failure\"}");
    std::fflush(stdout); std::_Exit(4);
}
