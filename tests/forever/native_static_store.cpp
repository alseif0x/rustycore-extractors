// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original VMapManager final-file lifecycle/query QA.
// Guard helpers derived from authored native_collision_query.cpp on this date.
// All manager/tree/model/math code is unchanged pin02245dcd245e7433e524577656177723d3e4992e.
// Original PERFORMANCE_PROFILING configuration disables logging, metrics and
// ordinary Trinity assertions; G3D release/NDEBUG and no-fast-math remain set.
// No helper/model/math stubs. Only reviewed synthetic complete positives execute
// AFTER strict Rust plan success. Invalid files and unsafe object segments never
// reach native. Root files remain read-only and are compared fully before/after.
// This controlled private QA wrapper does not claim adversarial path-race safety.
// CLI: native_static_store <vmaps-directory> <RCSTORE1-plan> <new-private-stage>.
// Plan: magic, u32 parentCount/opCount/options/reserved; parent rows map/parent/
// cosmetic (UINTMAX=None), then16u32/op. kinds0load1unloadtile2unloadmap3LOS4height5hit.
// Words0kind1map2x3y; float words4..6origin7..9end10maxOrModify;11ignore;12..15zero.
// Results RCSOUT01,u32count,reserved, then12words/op: kind,status,slotcount,
// activecount,globaluniqueModels,zero,payload4,zero,zero. Payload LOSbool/heightbits/
// hitboolXYZ; other words zero. Unloadstatus0 and loadstatus original LoadResult.
#include "VMapManager.h"
#include "ModelInstance.h"
#include "WorldModel.h"
#include <algorithm>
#include <array>
#include <bit>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <functional>
#include <limits>
#include <memory>
#include <span>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>
#ifndef PERFORMANCE_PROFILING
#error "QA requires the declared original instrumentation-disabled configuration"
#endif
#ifdef G3D_DEBUG
#error "QA requires pinned G3D release configuration"
#endif
namespace {
using Bytes = std::vector<unsigned char>;
using View = std::span<unsigned char const>;
using File = std::unique_ptr<FILE, int (*)(FILE*)>;
namespace fs = std::filesystem;
constexpr std::size_t MaxFile = 128 * 1024 * 1024;
constexpr std::size_t MaxQueries = 4096;
constexpr std::size_t MaxEnvelope = 24 + 64 * 12 + MaxQueries * 64;


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
fs::path output_path(fs::path const& requested, fs::path const& source) {
    auto leaf=requested.filename();
    if(leaf.empty()||leaf=="."||leaf=="..")fail("qa_output_boundary");
    auto parent=fs::canonical(requested.parent_path());
    for(auto p=parent;;p=p.parent_path()) {
        if(fs::exists(p/".git")||(fs::exists(p/"HEAD")&&fs::exists(p/"objects")&&fs::exists(p/"refs")))
            fail("qa_output_in_git");
        if(p==p.root_path())break;
    }
    auto stage=parent/leaf;
    auto relative=stage.lexically_relative(fs::canonical(source));
    if(!relative.empty()&&*relative.begin()!="..")fail("qa_output_in_source");
    return stage;
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
struct Parent { uint32 map, parent, cosmetic; };
struct Operation { std::array<uint32,16> words; };
float scalar(uint32 word) { return std::bit_cast<float>(word); }
Operation operation(Cursor& c) {
    Operation op{}; for (auto& w:op.words) w=c.word(); auto const& w=op.words;
    if(w[0]>5 || w[12] || w[13] || w[14] || w[15]) fail("qa_operation");
    if(w[0]<=2) {
        if(w[2]>32767 || w[3]>255 || (w[0]==2 && (w[2]||w[3]))) fail("qa_tile");
        for(unsigned i=4;i<12;++i) if(w[i]) fail("qa_reserved");
    } else {
        if(w[2]||w[3]||w[11]>1) fail("qa_query_options");
        for(unsigned i=4;i<10;++i) if(!std::isfinite(scalar(w[i]))) fail("qa_nonfinite");
        if(w[0]==3) { if(w[10]) fail("qa_reserved"); }
        else if(w[0]==4) {
            if(w[7]||w[8]||w[9]||w[11]||!std::isfinite(scalar(w[10]))||scalar(w[10])<0) fail("qa_height");
        } else {
            if(w[11]||!std::isfinite(scalar(w[10]))) fail("qa_hit");
            constexpr float mid=0.5f*64.f*533.33333333f;
            float dx=(mid-scalar(w[7]))-(mid-scalar(w[4]));
            float dy=(mid-scalar(w[8]))-(mid-scalar(w[5]));
            float dz=scalar(w[9])-scalar(w[6]);
            float length=std::sqrt((dx*dx+dy*dy)+dz*dz);
            if(!std::isfinite(length)||length>=std::numeric_limits<float>::max()) fail("qa_hit_length");
        }
    }
    return op;
}
struct Asset {
    Snapshot snapshot;
    Bytes bytes;
    explicit Asset(fs::path path):snapshot(std::move(path),MaxFile),bytes(snapshot.read()) {}
};
struct Inputs {
    fs::path root;
    std::vector<Asset> assets;
    std::vector<std::pair<fs::path,struct stat>> directories;
    explicit Inputs(fs::path r):root(std::move(r)) {
        struct stat info{};
        if(::lstat(root.c_str(),&info)||!S_ISDIR(info.st_mode)) fail("qa_source_directory");
        directories.emplace_back(root,info); std::size_t total=0;
        for(auto const& e:fs::recursive_directory_iterator(root)) {
            auto relative=e.path().lexically_relative(root);
            if(std::distance(relative.begin(),relative.end())>2) fail("qa_source_depth");
            struct stat named{}; if(::lstat(e.path().c_str(),&named)) fail("qa_source_io");
            if(S_ISDIR(named.st_mode)) {
                if(directories.size()>=65) fail("qa_source_directories");
                directories.emplace_back(e.path(),named);
            } else {
                if(!S_ISREG(named.st_mode)||assets.size()>=1024) fail("qa_source_file");
                assets.emplace_back(e.path());add(total,assets.back().bytes.size(),MaxFile);
            }
        }
        verify();
    }
    void verify() const {
        for(auto const& [path,before]:directories) {
            struct stat after{};if(::lstat(path.c_str(),&after)||!same(before,after)) fail("qa_source_changed");
        }
        std::size_t count=0;
        for(auto const& e:fs::recursive_directory_iterator(root)) { (void)e; ++count; }
        if(count!=assets.size()+directories.size()-1) fail("qa_source_inventory_changed");
        for(auto const& a:assets) a.snapshot.matches(a.bytes);
    }
};
struct Manager {
    VMAP::VMapManager value;
    std::vector<uint32> ids;
    // Explicitly unload before VMapManager fields (including its mutex) destroy.
    // No borrowed WorldModel pointer/shared_ptr escapes the active operation.
    ~Manager() { for(auto id:ids)value.unloadMap(id); }
};
std::array<uint32,3> counts(Manager const& manager,uint32 map) {
    auto own=manager.value.getModelsOnMap(map);
    std::size_t active=0;
    for(auto const& model:own) active+=model.getWorldModel()!=nullptr;
    std::vector<VMAP::WorldModel const*> models;
    for(auto id:manager.ids) for(auto const& model:manager.value.getModelsOnMap(id)) {
        if(models.size()>=65536) fail("qa_model_count");
        if(auto ptr=model.getWorldModel())models.push_back(ptr);
    }
    std::sort(models.begin(),models.end(),std::less<VMAP::WorldModel const*>());
    models.erase(std::unique(models.begin(),models.end()),models.end());
    if(own.size()>65536) fail("qa_slot_count");
    return {uint32(own.size()),uint32(active),uint32(models.size())};
}
} // namespace
int main(int argc,char** argv) try {
    static_assert(std::endian::native==std::endian::little && sizeof(float)==4);
    static_assert(std::numeric_limits<float>::is_iec559);
    if(argc!=4) {std::puts("{\"status\":\"error\",\"code\":\"qa_usage\"}");return 2;}
    ::umask(0077);
    Snapshot plan(fs::absolute(argv[2]),MaxEnvelope);auto bytes=plan.read();Cursor c{bytes};
    if(std::memcmp(c.take(8).data(),"RCSTORE1",8))fail("qa_magic");
    auto parents=c.word(),operations=c.word(),options=c.word(),reserved=c.word();
    if(!parents||parents>64||operations>MaxQueries||options>7||reserved)fail("qa_plan_limits");
    std::vector<Parent> rows;std::unordered_map<uint32,std::vector<uint32>> topology;
    for(uint32 i=0;i<parents;++i) {
        Parent row{c.word(),c.word(),c.word()};
        if((!rows.empty()&&rows.back().map>=row.map)||(row.parent!=UINT32_MAX&&row.cosmetic!=UINT32_MAX&&row.parent!=row.cosmetic))fail("qa_topology");
        rows.push_back(row);topology.emplace(row.map,std::vector<uint32>{});
    }
    auto find=[&](uint32 id)->Parent const& {
        auto it=std::lower_bound(rows.begin(),rows.end(),id,[](Parent const& p,uint32 v){return p.map<v;});
        if(it==rows.end()||it->map!=id)fail("qa_unknown_map");return *it;
    };
    for(auto const& row:rows) {
        auto parent=row.parent!=UINT32_MAX?row.parent:row.cosmetic;
        if(parent!=UINT32_MAX) {find(parent);topology[parent].push_back(row.map);}
        unsigned depth=0;
        while(parent!=UINT32_MAX) {
            if(parent==row.map||++depth>parents)fail("qa_cycle");
            auto const& p=find(parent);parent=p.parent!=UINT32_MAX?p.parent:p.cosmetic;
        }
    }
    auto op_start=c.at;
    for(uint32 i=0;i<operations;++i) {auto op=operation(c);find(op.words[1]);}
    if(c.at!=bytes.size())fail("qa_tail");
    Inputs source(fs::absolute(argv[1]));auto stage=output_path(fs::absolute(argv[3]),source.root);directory(stage);
    Bytes result{'R','C','S','O','U','T','0','1'};push(result,operations);push(result,0);
    std::size_t loads=0,unloads=0,queries=0;
    {
        Capture capture(stage/"native.log");Manager manager;
        for(auto const& row:rows)manager.ids.push_back(row.map);
        manager.value.InitializeThreadUnsafe(topology);
        manager.value.LoadPathOnlyModels=bool(options&1);
        manager.value.setEnableLineOfSightCalc(bool(options&2));
        manager.value.setEnableHeightCalc(bool(options&4));
        auto base=source.root.string()+"/";c.at=op_start;
        for(uint32 i=0;i<operations;++i) {
            auto op=operation(c);auto const& w=op.words;uint32 status=0;std::array<uint32,4> payload{};
            if(w[0]==0) {status=static_cast<uint32>(manager.value.loadMap(base,w[1],w[2],w[3]));++loads;}
            else if(w[0]==1) {manager.value.unloadMap(w[1],w[2],w[3]);++unloads;}
            else if(w[0]==2) {manager.value.unloadMap(w[1]);++unloads;}
            else if(w[0]==3) {
                payload[0]=manager.value.isInLineOfSight(w[1],scalar(w[4]),scalar(w[5]),scalar(w[6]),scalar(w[7]),scalar(w[8]),scalar(w[9]),static_cast<VMAP::ModelIgnoreFlags>(w[11]));++queries;
            } else if(w[0]==4) {
                payload[0]=std::bit_cast<uint32>(manager.value.getHeight(w[1],scalar(w[4]),scalar(w[5]),scalar(w[6]),scalar(w[10])));++queries;
            } else {
                float x=0,y=0,z=0;
                payload[0]=manager.value.getObjectHitPos(w[1],scalar(w[4]),scalar(w[5]),scalar(w[6]),scalar(w[7]),scalar(w[8]),scalar(w[9]),x,y,z,scalar(w[10]));
                payload[1]=std::bit_cast<uint32>(x);payload[2]=std::bit_cast<uint32>(y);payload[3]=std::bit_cast<uint32>(z);++queries;
            }
            auto state=counts(manager,w[1]);push(result,w[0]);push(result,status);
            for(auto value:state)push(result,value);push(result,0);
            for(auto value:payload)push(result,value);push(result,0);push(result,0);
        }
        for(auto id:manager.ids)manager.value.unloadMap(id);
        for(auto id:manager.ids)if(!manager.value.getModelsOnMap(id).empty())fail("qa_cleanup");
        source.verify();plan.matches(bytes);capture.finish();
    }
    write_new(stage/"results.bin",result);source.verify();plan.matches(bytes);
    if(std::distance(fs::directory_iterator(stage),fs::directory_iterator())!=2)fail("qa_inventory");
    std::printf("{\"status\":\"success\",\"operations\":%u,\"loads\":%zu,\"unloads\":%zu,\"queries\":%zu}\n",operations,loads,unloads,queries);return 0;
} catch(Failure const& e) {std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n",e.code);return 4;}
catch(...) {std::puts("{\"status\":\"error\",\"code\":\"qa_native_failure\"}");return 4;}
