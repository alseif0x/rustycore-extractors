// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: original Detour consumer of complete toy product outputs.
// Only reviewed positives admitted by Rust product extract + verify are supplied.
// No malformed/native self-copy execution, game files, network or key handling.
#include "DetourNavMesh.h"
#include "DetourNavMeshQuery.h"
#include "DetourAlloc.h"
#include <bit>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <vector>
static_assert(std::endian::native == std::endian::little);
static_assert(sizeof(dtPolyRef)==8 && sizeof(dtNavMeshParams)==28 && DT_NAVMESH_VERSION==7);
static std::vector<unsigned char> bytes(char const* path, size_t maximum) {
    std::ifstream f(path,std::ios::binary|std::ios::ate);
    if(!f || f.tellg()<0 || size_t(f.tellg())>maximum)throw std::runtime_error("input");
    std::vector<unsigned char> b(size_t(f.tellg()));f.seekg(0);
    f.read(reinterpret_cast<char*>(b.data()),b.size());if(!f)throw std::runtime_error("read");return b;
}
static uint32_t word(unsigned char const* p){uint32_t n;std::memcpy(&n,p,4);return n;}
struct Result {dtStatus nearest[2]{},status{};dtPolyRef refs[2]{};int count{},offmesh{};bool reached{};};
static Result route(dtNavMesh const& nav,dtNavMeshQuery& query,float const* from,float const* to){
    Result r;dtQueryFilter filter;filter.setIncludeFlags(0xffff);filter.setExcludeFlags(0);
    float extents[3]={8,10,8},near[2][3]{};
    r.nearest[0]=query.findNearestPoly(from,extents,&filter,&r.refs[0],near[0]);
    r.nearest[1]=query.findNearestPoly(to,extents,&filter,&r.refs[1],near[1]);
    if(dtStatusFailed(r.nearest[0])||dtStatusFailed(r.nearest[1])||!r.refs[0]||!r.refs[1]||r.refs[0]==r.refs[1]){std::cerr<<"nearest statuses "<<r.nearest[0]<<" "<<r.nearest[1]<<" refs "<<r.refs[0]<<" "<<r.refs[1]<<"\n";throw std::runtime_error("island endpoints");}
    dtPolyRef path[4096]{};r.status=query.findPath(r.refs[0],r.refs[1],near[0],near[1],&filter,path,&r.count,4096);
    if(dtStatusFailed(r.status)||(r.status&(DT_OUT_OF_NODES|DT_BUFFER_TOO_SMALL)))throw std::runtime_error("query limit");
    r.reached=r.count>0&&path[r.count-1]==r.refs[1];
    for(int i=0;i<r.count;++i){dtMeshTile const* t;dtPoly const* p;if(dtStatusFailed(nav.getTileAndPolyByRef(path[i],&t,&p)))throw std::runtime_error("path ref");if(p->getType()==DT_POLYTYPE_OFFMESH_CONNECTION)++r.offmesh;}
    return r;
}
static void json(Result const& r){std::cout<<"{\"reached\":"<<(r.reached?"true":"false")<<",\"path_polys\":"<<r.count<<",\"offmesh_polys\":"<<r.offmesh<<",\"status\":"<<r.status<<",\"nearest_status\":["<<r.nearest[0]<<","<<r.nearest[1]<<"],\"endpoint_refs\":["<<r.refs[0]<<","<<r.refs[1]<<"]}";}
int main(int argc,char** argv)try{
    if(argc!=4)throw std::runtime_error("usage: global tile source-xyz-bits");
    auto global=bytes(argv[1],40+44*1024),tile=bytes(argv[2],64*1024*1024+20),q=bytes(argv[3],24);
    if(global.size()<40||word(global.data())!=0x4d4d4150||word(global.data()+4)!=16||global.size()!=40+44*word(global.data()+36)||q.size()!=24||tile.size()<120)throw std::runtime_error("framing");
    dtNavMeshParams p{};std::memcpy(&p,global.data()+8,sizeof(p));dtNavMesh nav;if(dtStatusFailed(nav.init(&p)))throw std::runtime_error("init");
    auto data=static_cast<unsigned char*>(dtAlloc(tile.size()-20,DT_ALLOC_PERM));if(!data)throw std::bad_alloc();std::memcpy(data,tile.data()+20,tile.size()-20);
    if(dtStatusFailed(nav.addTile(data,tile.size()-20,DT_TILE_FREE_DATA,0,nullptr))){dtFree(data);throw std::runtime_error("tile load");}
    dtNavMeshQuery query;if(dtStatusFailed(query.init(&nav,4096)))throw std::runtime_error("query init");
    float a[3]={std::bit_cast<float>(word(q.data()+4)),std::bit_cast<float>(word(q.data()+8)),std::bit_cast<float>(word(q.data()))};
    float b[3]={std::bit_cast<float>(word(q.data()+16)),std::bit_cast<float>(word(q.data()+20)),std::bit_cast<float>(word(q.data()+12))};
    int connections=0;for(int i=0;i<nav.getMaxTiles();++i){auto t=static_cast<dtNavMesh const&>(nav).getTile(i);if(t&&t->header)connections+=t->header->offMeshConCount;}
    auto forward=route(nav,query,a,b),reverse=route(nav,query,b,a);
    std::cout<<"{\"original_detour\":true,\"stored_connections\":"<<connections<<",\"forward\":";json(forward);std::cout<<",\"reverse\":";json(reverse);std::cout<<"}\n";return 0;
}catch(std::exception const& e){std::cerr<<e.what()<<"\n";return 4;}
