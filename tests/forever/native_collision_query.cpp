// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original VMO/instance ray consumer synthetic QA.
// Guard utilities derived from authored native_collision_world.cpp on this date.
// Pin02245dcd245e7433e524577656177723d3e4992e. WorldModel::readFile,
// IntersectRay, ModelSpawn::readFromFile, ModelInstance::intersectRay and all
// BIH/triangle/G3D math remain unchanged original external sources/notices.
// No substitute geometry or traversal is linked. This is supplied complete VMO
// consumer evidence, not StaticMapTree/VMapManager or RustyCore integration.
// CLI: native_collision_query <inventory.bin> <new-private-stage>
// Input: RCQUERY1, u32vmoLen/spawnLen/queryCount/reserved0; VMO, optional one
// bounded ModelSpawn, then36byte queries: mode,ignore,originXYZ,directionXYZ,max.
// Float fields retain IEEE bits. Outputresults.bin: RCRSLT01,count,reserved0,
// then pairs of u32hit,floatdistancebits; miss preserves original maxdistance.
// Only independently validated synthetic positives may execute native readers;
// complete borrowed Rust validation and query success must precede this process.
// Infinite-distance native fixtures require a separate safe leaf-tree proof;
// original unary absent edges can be unsafe for infinity and stay Rust-only.
// Native positive scales may be negative/nonzero; source ray directions are
// finite/nonzero, never silently normalized. Parent supplies process/networkcaps.
// Private0700/0600 outputs with source/copy byte+identity guards. Native errors
// may leave a failed private stage. No publication or real-client claim.
#include "WorldModel.h"
#include "ModelInstance.h"
#include "ModelIgnoreFlags.h"
#include <algorithm>
#include <array>
#include <bit>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <limits>
#include <memory>
#include <span>
#include <string>
#include <utility>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>
#ifdef G3D_DEBUG
#error "native query QA requires pinned G3D release configuration"
#endif
namespace {
using Bytes = std::vector<unsigned char>;
using View = std::span<unsigned char const>;
using File = std::unique_ptr<FILE, int (*)(FILE*)>;
namespace fs = std::filesystem;
constexpr std::size_t MaxFile = 128 * 1024 * 1024;
constexpr std::size_t MaxQueries = 65536;
constexpr std::size_t MaxEnvelope = MaxFile + 1024 + MaxQueries * 36 + 24;


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

void push(Bytes& out, uint32 value) {
    for (unsigned shift : {0u, 8u, 16u, 24u}) out.push_back(static_cast<unsigned char>(value >> shift));
}
struct Query { uint32 mode, ignore; std::array<float,3> origin, direction; float maximum; };
Query read_query(Cursor& c) {
    Query q{}; q.mode=c.word(); q.ignore=c.word();
    for (auto& v:q.origin) v=std::bit_cast<float>(c.word());
    for (auto& v:q.direction) v=std::bit_cast<float>(c.word());
    q.maximum=std::bit_cast<float>(c.word());
    if (q.mode>1 || q.ignore>1 || std::isnan(q.maximum) || q.maximum<0
        || !std::all_of(q.origin.begin(),q.origin.end(),[](float v){return std::isfinite(v);})
        || !std::all_of(q.direction.begin(),q.direction.end(),[](float v){return std::isfinite(v);})
        || std::all_of(q.direction.begin(),q.direction.end(),[](float v){return v==0;})) fail("qa_query");
    return q;
}
} // namespace
int main(int argc, char** argv) try {
    static_assert(std::endian::native==std::endian::little && sizeof(float)==4 && sizeof(uint32)==4);
    static_assert(std::numeric_limits<float>::is_iec559);
    if(argc!=3) { std::puts("{\"status\":\"error\",\"code\":\"qa_usage\"}"); return 2; }
    ::umask(0077);
    Snapshot input(fs::absolute(argv[1]),MaxEnvelope);auto data=input.read();Cursor c{data};
    if(std::memcmp(c.take(8).data(),"RCQUERY1",8)) fail("qa_magic");
    auto vmo_len=c.word(),spawn_len=c.word(),count=c.word(),reserved=c.word();
    if(vmo_len<24 || vmo_len>MaxFile || spawn_len>1024 || count>MaxQueries || reserved) fail("qa_limits");
    auto vmo=c.take(vmo_len),spawn=c.take(spawn_len);auto query_start=c.at;
    for(uint32 i=0;i<count;++i) read_query(c);
    if(c.at!=data.size()) fail("qa_tail");
    if(std::memcmp(vmo.data(),"VMAP_4.E",8)) fail("qa_vmo_magic");
    auto stage=fs::absolute(argv[2]);directory(stage);
    write_new(stage/"model.vmo",vmo);Snapshot model_file(stage/"model.vmo",MaxFile);
    std::unique_ptr<Snapshot> spawn_file;
    if(spawn_len) {write_new(stage/"spawn.bin",spawn);spawn_file=std::make_unique<Snapshot>(stage/"spawn.bin",1024);}
    Bytes result{'R','C','R','S','L','T','0','1'};push(result,count);push(result,0);
    std::size_t hits=0;
    {
        Capture capture(stage/"native.log");
        auto model=std::make_shared<VMAP::WorldModel>();
        if(!model->readFile((stage/"model.vmo").string())) fail("qa_model_read");
        std::unique_ptr<VMAP::ModelInstance> instance;
        if(spawn_file) {
            VMAP::ModelSpawn placement{};
            if(!VMAP::ModelSpawn::readFromFile(spawn_file->file.get(),placement)) fail("qa_spawn_read");
            eof(spawn_file->file.get());
            if(!(placement.flags&VMAP::MOD_HAS_BOUND) || !std::isfinite(placement.iScale)
                || placement.iScale==0 || !std::isfinite(1.f/placement.iScale)) fail("qa_spawn_scale");
            instance=std::make_unique<VMAP::ModelInstance>(placement,model);
        }
        c.at=query_start;
        for(uint32 i=0;i<count;++i) {
            auto q=read_query(c);
            G3D::Ray ray(G3D::Vector3(q.origin[0],q.origin[1],q.origin[2]),G3D::Vector3(q.direction[0],q.direction[1],q.direction[2]));
            float distance=q.maximum;
            auto ignore=static_cast<VMAP::ModelIgnoreFlags>(q.ignore);
            bool hit=instance ? instance->intersectRay(ray,distance,q.mode!=0,ignore)
                              : model->IntersectRay(ray,distance,q.mode!=0,ignore);
            if(!hit && std::bit_cast<uint32>(distance)!=std::bit_cast<uint32>(q.maximum)) fail("qa_miss_distance");
            hits+=hit;push(result,hit?1:0);push(result,std::bit_cast<uint32>(distance));
        }
        model_file.matches(vmo);if(spawn_file)spawn_file->matches(spawn);input.matches(data);
        capture.finish();
    }
    write_new(stage/"results.bin",result);input.matches(data);model_file.matches(vmo);if(spawn_file)spawn_file->matches(spawn);
    auto expected=spawn_len?4:3;
    if(std::distance(fs::directory_iterator(stage),fs::directory_iterator())!=expected) fail("qa_inventory");
    std::printf("{\"status\":\"success\",\"queries\":%u,\"hits\":%zu,\"misses\":%zu}\n",count,hits,std::size_t(count)-hits);
    return 0;
} catch(Failure const& e) {
    std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n",e.code);return 4;
} catch(...) { std::puts("{\"status\":\"error\",\"code\":\"qa_native_failure\"}");return 4; }
