// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only adapter to unmodified pinned wmo.cpp.
// Original source/notices are materialized externally, not imported here.
// Execute original WMORoot::open/header writer and WMOGroup's parser,
// GetLiquidTypeId, ShouldSkip and writer. MOHD/MOGN/GFID are independently
// framed from the synthetic fixture. The complete exporter, doodad
// extraction, CASC and assembler are not executed.
//
// CASCFile below is an authored complete-memory input adapter, with the
// original header's class layout. IsLiquidIgnored is an authored synthetic
// map predicate. Neither implements archive acquisition or DB2 decoding.
// Assertions link the original Errors.cpp; no writer/geometry code is replaced.
// Only prevalidated synthetic positives belong in this native caller.
//
// CLI: native_wmo_conversion <fixture> <new-output>
// RCEWM001 + eight LE u32: rootFlags, rootID, declaredRootGroups,
// groupNameBytes, inputGroups, typeRows, materialRows, outputCap.
// Then groupNameBytes; typeRows*(u32 ID,u8 material);
// materialRows*(u32 ID,u32 flags); inputGroups*(u32 FDID,u32 size,bytes).
// Groups are already in detailed GFID order. All outputs use exclusive create.

#include "wmo.h"
#include "cascfile.h"
#include "vmapexport.h"
#include <algorithm>
#include <bit>
#include <cerrno>
#include <cstdio>
#include <cstring>
#include <filesystem>
#include <limits>
#include <map>
#include <span>
#include <string>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

namespace {
using Bytes = std::vector<unsigned char>;
constexpr std::size_t Limit = 64 * 1024 * 1024;
Bytes rootInput;
std::vector<Bytes> inputs;
std::map<uint32, uint8> types;
std::map<uint32, uint32> materials;
std::size_t ioOpens = 0;
std::size_t rootOpens = 0;
std::size_t doodadCalls = 0;
void wordAt(Bytes& bytes, std::size_t at, uint32 value) {
    if (at > bytes.size() || bytes.size()-at < 4) throw "qa_root_extent";
    for (unsigned i=0;i<4;++i) bytes[at+i]=static_cast<unsigned char>(value>>(8*i));
}
void frame(char const* tag, std::span<unsigned char const> bytes) {
    if (rootInput.size()>Limit-8 || bytes.size()>Limit-8-rootInput.size()) throw "qa_root_limit";
    auto at=rootInput.size();rootInput.resize(at+8+bytes.size());
    for (unsigned i=0;i<4;++i) rootInput[at+i]=tag[3-i];
    wordAt(rootInput,at+4,static_cast<uint32>(bytes.size()));
    std::copy(bytes.begin(),bytes.end(),rootInput.begin()+at+8);
}
bool unchanged(struct stat const& a, struct stat const& b) {
    return a.st_dev==b.st_dev && a.st_ino==b.st_ino && a.st_mode==b.st_mode
        && a.st_size==b.st_size && a.st_mtim.tv_sec==b.st_mtim.tv_sec
        && a.st_mtim.tv_nsec==b.st_mtim.tv_nsec && a.st_ctim.tv_sec==b.st_ctim.tv_sec
        && a.st_ctim.tv_nsec==b.st_ctim.tv_nsec;
}
Bytes readComplete(char const* path) {
    struct File { int fd; ~File(){if(fd>=0)::close(fd);} } file{::open(path,O_RDONLY|O_CLOEXEC|O_NOFOLLOW|O_NONBLOCK)};
    struct stat before{},after{},named{};
    if(file.fd<0 || ::fstat(file.fd,&before) || !S_ISREG(before.st_mode))throw "qa_input_io";
    if(before.st_size<0 || std::uintmax_t(before.st_size)>Limit)throw "qa_input_limit";
    Bytes bytes(static_cast<std::size_t>(before.st_size));
    std::size_t at=0;
    while(at<bytes.size()){
        auto got=::read(file.fd,bytes.data()+at,bytes.size()-at);
        if(got<0 && errno==EINTR)continue;
        if(got<=0)throw "qa_input_read";
        at+=got;
    }
    unsigned char tail;ssize_t extra;
    do{extra=::read(file.fd,&tail,1);}while(extra<0 && errno==EINTR);
    if(extra!=0 || ::fstat(file.fd,&after) || ::lstat(path,&named)
        || !unchanged(before,after) || !unchanged(before,named))throw "qa_input_changed";
    return bytes;
}
struct Input {
    Bytes bytes;
    std::size_t at = 0;
    std::span<unsigned char const> take(std::size_t size) {
        if (at > bytes.size() || size > bytes.size() - at) throw "qa_input_extent";
        auto result = std::span(bytes).subspan(at, size); at += size; return result;
    }
    uint32 word() { auto b=take(4); return uint32(b[0]) | uint32(b[1])<<8 | uint32(b[2])<<16 | uint32(b[3])<<24; }
};
}

std::shared_ptr<CASC::Storage> CascStorage;
CASCFile::CASCFile(std::shared_ptr<CASC::Storage const>, char const* filename, bool)
    : eof(false), buffer(nullptr), pointer(0), size(0) {
    Bytes const* bytes;
    if (std::strcmp(filename,"root")==0) { bytes=&rootInput; ++rootOpens; }
    else {
        std::size_t parsed = 0;
        auto ordinal = std::stoul(filename, &parsed);
        if (parsed != std::strlen(filename) || ordinal >= inputs.size()) throw "qa_input_name";
        bytes=&inputs[ordinal]; ++ioOpens;
    }
    size = bytes->size(); buffer = new char[size];
    if(size)std::memcpy(buffer, bytes->data(), size);
}
size_t CASCFile::read(void* destination, size_t count) {
    if (eof) return 0;
    // Unlike unchecked native short reads, malformed fixtures abort here.
    if (pointer > size || count > size-pointer) throw "qa_native_read_extent";
    if(count)std::memcpy(destination, buffer+pointer, count); pointer += count; return count;
}
void CASCFile::seek(int offset) {
    if (offset < 0 || static_cast<std::size_t>(offset) > size) throw "qa_native_seek_extent";
    pointer = offset; eof = pointer >= size;
}
void CASCFile::close() { delete[] buffer; buffer=nullptr; eof=true; }
bool IsLiquidIgnored(uint32 id) {
    auto type = types.find(id);
    if (type == types.end()) return false;
    auto material = materials.find(type->second);
    return material != materials.end() && (material->second & 2) != 0;
}
// Root fixtures have no MODN/MODI. Retained native references must never
// turn this parser comparison into an implicit model acquisition.
ExtractedModelData const* ExtractSingleModel(std::string&) {
    ++doodadCalls; throw "qa_unexpected_doodad";
}

int main(int argc, char** argv) try {
    static_assert(std::endian::native == std::endian::little);
    static_assert(sizeof(float)==4 && std::numeric_limits<float>::is_iec559);
    static_assert(sizeof(WMOLiquidHeader)==30 && sizeof(WMOLiquidVert)==8);
    static_assert(sizeof(int)==4 && sizeof(ModelFlags)==4);
    if (argc!=3 || std::filesystem::exists(argv[2])) throw "qa_invocation";
    Input input{readComplete(argv[1])};
    auto magic=input.take(8); if (std::memcmp(magic.data(),"RCEWM001",8)) throw "qa_input_magic";
    auto rootFlags=input.word(); if(rootFlags>65535)throw "qa_root_flags";
    auto rootId=input.word(), declaredGroups=input.word();
    auto names=input.word(), groups=input.word(), nt=input.word(), nm=input.word(), cap=input.word();
    if(names>65536 || groups>65536 || declaredGroups>1000000 || groups>declaredGroups || nt>10000 || nm>10000 || cap>Limit) throw "qa_counts";
    auto nameBytes=input.take(names);
    for(uint32 i=0;i<nt;++i){auto id=input.word();types[id]=input.take(1)[0];}
    for(uint32 i=0;i<nm;++i){auto id=input.word();materials[id]=input.word();}
    std::vector<uint32> ids;ids.reserve(groups);
    for(uint32 i=0;i<groups;++i){auto id=input.word();if(!id)throw "qa_fdid_zero";ids.push_back(id);auto n=input.word();auto b=input.take(n);inputs.emplace_back(b.begin(),b.end());}
    if(input.at!=input.bytes.size())throw "qa_input_tail";
    Bytes header(64);wordAt(header,4,declaredGroups);wordAt(header,32,rootId);wordAt(header,60,rootFlags);
    frame("MOHD",header);frame("MOGN",nameBytes);
    Bytes inventory(std::size_t(declaredGroups)*4*((rootFlags&0x10)?3:1));
    for(std::size_t i=0;i<ids.size();++i)wordAt(inventory,4*i,ids[i]);
    frame("GFID",inventory);
    WMORoot root("root");if(!root.open())throw "qa_root_open";
    if(root.flags!=rootFlags || root.RootWMOID!=rootId || root.nGroups!=declaredGroups || root.groupFileDataIDs!=ids || root.GroupNames.size()!=names || !std::equal(nameBytes.begin(),nameBytes.end(),root.GroupNames.begin(),[](unsigned char a,char b){return a==static_cast<unsigned char>(b);}))throw "qa_root_projection";
    std::vector<WMOGroup> parsed; parsed.reserve(groups);
    for(uint32 i=0;i<groups;++i){auto& group=parsed.emplace_back(std::to_string(i));if(!group.open(&root))throw "qa_group_open";}
    // No final output exists until all groups have been parsed and converted.
    auto stage=std::filesystem::path(argv[2]);stage += ".stage-XXXXXX";
    std::string stageName=stage.string();int descriptor=::mkstemp(stageName.data());if(descriptor<0)throw "qa_stage";
    FILE* output=::fdopen(descriptor,"w+b");if(!output){::close(descriptor);throw "qa_stage";}
    if(!root.ConvertToVMAPRootWmo(output))throw "qa_root_writer";
    uint32 triangles=0, emitted=0;
    for(auto& group:parsed){
        if(group.ShouldSkip(&root))continue;
        if((group.mogpFlags2&0x80) && group.parentOrFirstChildSplitGroupIndex>=0 && std::size_t(group.parentOrFirstChildSplitGroupIndex)<parsed.size())
            group.groupWMOID=parsed[group.parentOrFirstChildSplitGroupIndex].groupWMOID;
        auto count=group.ConvertToVMAPGroupWmo(output,false);if(count<0 || uint32(count)>INT32_MAX-triangles)throw "qa_triangle_limit";
        triangles+=count;++emitted;
    }
    if(std::fflush(output)||std::ferror(output))throw "qa_output_io";
    auto length=std::ftell(output);if(length<0||std::size_t(length)>cap)throw "qa_output_limit";
    if(std::fseek(output,8,SEEK_SET)||std::fwrite(&triangles,4,1,output)!=1||std::fwrite(&emitted,4,1,output)!=1||std::fflush(output)||::fsync(descriptor)||std::fclose(output))throw "qa_output_io";
    if(triangles && ::link(stageName.c_str(),argv[2]))throw "qa_output_exists";
    if(ioOpens!=groups || rootOpens!=1 || doodadCalls)throw "qa_open_count";
    std::printf("{\"status\":\"%s\",\"groups\":%u,\"triangles\":%u,\"bytes\":%ld,\"original_group_parses\":%zu,\"original_root_parses\":%zu,\"doodad_acquisition_calls\":%zu}\n",triangles?"complete":"no_collision",emitted,triangles,triangles?length:0,ioOpens,rootOpens,doodadCalls);
    return 0;
} catch(char const* code) { std::printf("{\"status\":\"error\",\"code\":\"%s\"}\n",code);return 4; }
catch(...) { std::printf("{\"status\":\"error\",\"code\":\"qa_exception\"}\n");return 4; }
