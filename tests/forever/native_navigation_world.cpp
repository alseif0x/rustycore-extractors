// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original Detour multi-tile lifecycle/query acceptance.
// Trusted, previously verified complete synthetic MMap files only. Bounded QA;
// no client acquisition, native malformed-parser parity or runtime activation.
#define main unused_navigation_detour_main
#include "native_detour.cpp"
#undef main
#include <algorithm>
#include <filesystem>
#include <set>
#include <map>
#include <string>
#include <tuple>
#include <array>
#include <iostream>
struct Blob {std::vector<unsigned char> bytes;};
static std::vector<unsigned char> read(std::filesystem::path const& p) {
    std::ifstream f(p,std::ios::binary);if(!f)throw std::runtime_error("read");
    std::vector<unsigned char>b((std::istreambuf_iterator<char>(f)),{});if(b.size()>64*1024*1024)throw std::runtime_error("size");return b;
}
static uint32_t word(std::vector<unsigned char>const&b,size_t i){uint32_t v;std::memcpy(&v,b.data()+i,4);return v;}
static float number(std::vector<unsigned char>const&b,size_t i){return std::bit_cast<float>(word(b,i));}
static void append(std::vector<uint32_t>&a,std::vector<uint32_t>const&b){a.insert(a.end(),b.begin(),b.end());}
static void status(std::vector<uint32_t>&out,dtStatus s){out.push_back(dtStatusFailed(s)?s:0);}
struct Pair {std::array<float,3>a,b;};
static std::array<float,3> center(dtMeshTile const*t,unsigned p){
    if(!t||!t->header||p>=unsigned(t->header->offMeshBase))throw std::runtime_error("poly");auto poly=&t->polys[p];std::array<float,3>v{};
    for(int i=0;i<poly->vertCount;++i)for(int k=0;k<3;++k)v[k]+=t->verts[poly->verts[i]*3+k];for(auto&n:v)n/=poly->vertCount;return v;
}
static std::vector<uint32_t> queries(dtNavMesh const& nav,std::vector<Pair>const&pairs) {
    std::vector<uint32_t>out;dtNavMeshQuery q;auto s=q.init(&nav,4096);status(out,s);if(dtStatusFailed(s))throw std::runtime_error("query init");dtQueryFilter filter;filter.setIncludeFlags(65535);filter.setExcludeFlags(0);out.push_back(pairs.size());
    for(auto const&p:pairs){
        float extent[3]={1.0f,6.0f,1.0f},near[2][3]{};dtPolyRef refs[2]{};auto a=p.a.data(),b=p.b.data();
        for(int i=0;i<2;++i){s=q.findNearestPoly(i?b:a,extent,&filter,&refs[i],near[i]);status(out,s);reference(out,refs[i]);point(out,near[i]);}
        dtPolyRef path[4096]{};int count=0;s=q.findPath(refs[0],refs[1],near[0],near[1],&filter,path,&count,4096);status(out,s);out.push_back(count);for(int i=0;i<count;++i)reference(out,path[i]);
        float straight[4096*3]{};unsigned char flags[4096]{};dtPolyRef sr[4096]{};int n=0;s=q.findStraightPath(near[0],near[1],path,count,straight,flags,sr,&n,4096);status(out,s);out.push_back(n);for(int i=0;i<n;++i){point(out,straight+i*3);out.push_back(flags[i]);reference(out,sr[i]);}
        out.push_back(count&&path[count-1]==refs[1]);std::set<unsigned>tiles;for(int i=0;i<count;++i)tiles.insert(nav.decodePolyIdTile(path[i]));out.push_back(tiles.size());
    }
    return out;
}
int main(int argc,char**argv)try {
    if(argc!=4)throw std::runtime_error("usage: plan.txt oracle.bin queries.bin");auto root=std::filesystem::canonical(std::filesystem::path(argv[1]).parent_path());std::ifstream f(argv[1]);std::string global;unsigned n; if(!(f>>global>>n)||n>16)throw std::runtime_error("plan");
    auto g=read(root/global);if(g.size()<40||word(g,0)!=0x4d4d4150||word(g,4)!=16||g.size()!=40+uint64_t(word(g,36))*44)throw std::runtime_error("global");dtNavMeshParams params{};for(int i=0;i<3;++i)params.orig[i]=number(g,8+i*4);params.tileWidth=number(g,20);params.tileHeight=number(g,24);params.maxTiles=word(g,28);params.maxPolys=word(g,32);if(params.maxTiles<1||params.maxTiles>4096)throw std::runtime_error("capacity");
    std::vector<Blob>blobs;for(unsigned i=0;i<n;++i){std::string path;if(!(f>>path))throw std::runtime_error("tile path");auto b=read(root/path);if(b.size()<120||word(b,0)!=0x4d4d4150||word(b,4)!=7||word(b,8)!=16||word(b,12)!=b.size()-20||word(b,20)!=0x444e4156)throw std::runtime_error("tile");blobs.push_back({std::move(b)});}std::string tail;if(f>>tail)throw std::runtime_error("tail");
    dtNavMesh mesh;std::vector<uint32_t>out{0x31574e52};auto s=mesh.init(&params);status(out,s);if(dtStatusFailed(s))throw std::runtime_error("nav init");out.push_back(mesh.getMaxTiles());out.push_back(n);std::vector<dtTileRef>refs(n);unsigned loaded=0;
    auto add=[&](unsigned i){auto const&b=blobs[i].bytes;auto data=static_cast<unsigned char*>(dtAlloc(b.size()-20,DT_ALLOC_PERM));if(!data)throw std::bad_alloc();std::memcpy(data,b.data()+20,b.size()-20);dtTileRef ref=0;auto st=mesh.addTile(data,b.size()-20,DT_TILE_FREE_DATA,0,&ref);if(dtStatusFailed(st))dtFree(data);status(out,st);reference(out,ref);if(dtStatusSucceed(st)){refs[i]=ref;++loaded;}return st;};
    for(unsigned i=0;i<n;++i)add(i);
    // Select explicit cross-tile witnesses from actual external polygon links.
    // Endpoint centroids and native outcomes are retained in a separate plan;
    // the unchanged RustyCore public API receives exactly these positions.
    std::vector<Pair>pairs;std::set<std::pair<dtPolyRef,dtPolyRef>>seen;std::map<std::pair<unsigned,unsigned>,unsigned>sampled;unsigned external=0;
    for(int i=0;i<mesh.getMaxTiles();++i){auto tile=static_cast<dtNavMesh const&>(mesh).getTile(i);if(!tile||!tile->header||!tile->header->offMeshBase)continue;
        if(pairs.size()<48)pairs.push_back({center(tile,0),center(tile,tile->header->offMeshBase-1)});
        for(int p=0;p<tile->header->offMeshBase;++p)for(unsigned j=tile->polys[p].firstLink;j!=DT_NULL_LINK;j=tile->links[j].next){if(j>=unsigned(tile->header->maxLinkCount))throw std::runtime_error("link");auto ref=tile->links[j].ref;if(!ref||mesh.decodePolyIdTile(ref)==unsigned(i))continue;auto target=static_cast<dtNavMesh const&>(mesh).getTile(mesh.decodePolyIdTile(ref));auto poly=mesh.decodePolyIdPoly(ref);if(!target||!target->header||poly>=unsigned(target->header->offMeshBase))continue;++external;auto from=mesh.getPolyRefBase(tile)|dtPolyRef(p);auto key=std::minmax(from,ref);auto tilePair=std::make_pair(std::min(unsigned(i),mesh.decodePolyIdTile(ref)),std::max(unsigned(i),mesh.decodePolyIdTile(ref)));if(pairs.size()<48&&sampled[tilePair]<4&&seen.insert(key).second){++sampled[tilePair];pairs.push_back({center(tile,p),center(target,poly)});}}
    }
    // Disconnected and absent-nearest cases, without manufacturing a new mesh.
    std::vector<dtMeshTile const*>tiles;for(int i=0;i<mesh.getMaxTiles();++i){auto t=static_cast<dtNavMesh const&>(mesh).getTile(i);if(t&&t->header&&t->header->offMeshBase)tiles.push_back(t);}
    if(tiles.size()>1&&pairs.size()<63)pairs.push_back({center(tiles.front(),0),center(tiles.back(),0)});
    pairs.push_back({{1000000.0f,0.0f,1000000.0f},{1000001.0f,0.0f,1000001.0f}});
    std::vector<uint32_t>plan{0x31514e52,unsigned(pairs.size()),external};for(auto const&p:pairs){point(plan,p.a.data());point(plan,p.b.data());}save(argv[3],plan.data(),plan.size()*4);
    append(out,queries(mesh,pairs));
    // Duplicate occupied coordinates must reject without altering loaded state.
    auto first=std::find_if(refs.begin(),refs.end(),[](auto v){return v!=0;});unsigned index=first-refs.begin();dtTileRef old=first!=refs.end()?*first:0;
    out.push_back(old?1:0);
    if(old){auto prev=refs[index];auto dup=add(index);if(dtStatusSucceed(dup))throw std::runtime_error("duplicate admitted");refs[index]=prev;
        status(out,mesh.removeTile(old,nullptr,nullptr));refs[index]=0;--loaded;append(out,queries(mesh,pairs));
        auto re=add(index);if(dtStatusFailed(re)||refs[index]==old)throw std::runtime_error("reload");append(out,queries(mesh,pairs));status(out,mesh.removeTile(old,nullptr,nullptr));}
    out.push_back(loaded);for(unsigned i=n;i>0;--i)if(refs[i-1]){status(out,mesh.removeTile(refs[i-1],nullptr,nullptr));refs[i-1]=0;}append(out,queries(mesh,pairs));save(argv[2],out.data(),out.size()*4);return 0;
}catch(std::exception const&e){std::cerr<<e.what()<<"\n";return 5;}
