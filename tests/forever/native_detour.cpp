// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original Detour construction, load and bounded queries.
#include "DetourNavMeshBuilder.h"
#include "DetourNavMeshQuery.h"
#include "DetourAlloc.h"
#include <bit>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <memory>
#include <stdexcept>
#include <vector>
static_assert(std::endian::native==std::endian::little);
static_assert(sizeof(dtPolyRef)==8 && sizeof(dtMeshHeader)==100 && sizeof(dtPoly)==32 && sizeof(dtLink)==16 && sizeof(dtPolyDetail)==12 && sizeof(dtBVNode)==16 && sizeof(dtOffMeshConnection)==36);
static_assert(DT_NAVMESH_VERSION==7 && DT_VERTS_PER_POLYGON==6);
struct Input {
    std::ifstream f;explicit Input(char const* p):f(p,std::ios::binary){if(!f)throw std::runtime_error("input");}
    uint32_t word(){uint32_t n=0;f.read(reinterpret_cast<char*>(&n),4);if(!f)throw std::runtime_error("truncated");return n;}
    float number(){return std::bit_cast<float>(word());}
};
void save(char const* path,void const* data,size_t bytes){std::ofstream out(path,std::ios::binary);out.write(static_cast<char const*>(data),bytes);if(!out)throw std::runtime_error("output");}
void reference(std::vector<uint32_t>& out,dtPolyRef ref){out.push_back(uint32_t(ref));out.push_back(uint32_t(ref>>32));}
void point(std::vector<uint32_t>& out,float const* p){for(int i=0;i<3;++i)out.push_back(std::bit_cast<uint32_t>(p[i]));}
std::vector<uint32_t> inspect(unsigned char const* source,int size,unsigned probe){
    std::vector<uint32_t> out;auto data=static_cast<unsigned char*>(dtAlloc(size,DT_ALLOC_PERM));if(!data)throw std::bad_alloc();std::memcpy(data,source,size);dtNavMesh nav;
    dtStatus status=nav.init(data,size,DT_TILE_FREE_DATA);out.push_back(status);if(dtStatusFailed(status)){dtFree(data);return out;}
    auto tile=static_cast<dtNavMesh const&>(nav).getTile(0);if(!tile||!tile->header)throw std::runtime_error("tile");auto h=tile->header;
    out.insert(out.end(),{uint32_t(h->version),uint32_t(h->polyCount),uint32_t(h->vertCount),uint32_t(h->offMeshConCount),uint32_t(h->detailMeshCount)});
    size_t links=0;for(int i=0;i<h->polyCount;++i){int visited=0;for(unsigned j=tile->polys[i].firstLink;j!=DT_NULL_LINK;j=tile->links[j].next){if(j>=unsigned(h->maxLinkCount)||++visited>h->maxLinkCount)throw std::runtime_error("link chain");++links;}}
    out.push_back(uint32_t(links));out.push_back(probe);if(!probe)return out;
    dtNavMeshQuery query;status=query.init(&nav,4096);out.push_back(status);if(dtStatusFailed(status))return out;dtQueryFilter filter;filter.setIncludeFlags(0xffff);filter.setExcludeFlags(0);
    float centers[2][3]{};for(int side=0;side<2;++side){auto p=&tile->polys[((side!=0)^(probe==2))?h->offMeshBase-1:0];for(int j=0;j<p->vertCount;++j)for(int k=0;k<3;++k)centers[side][k]+=tile->verts[p->verts[j]*3+k];for(float& n:centers[side])n/=p->vertCount;}
    float extents[3]={2.0f,h->walkableHeight+h->walkableClimb+2.0f,2.0f};float near[2][3]{};dtPolyRef refs[2]{};
    for(int i=0;i<2;++i){status=query.findNearestPoly(centers[i],extents,&filter,&refs[i],near[i]);out.push_back(status);reference(out,refs[i]);point(out,near[i]);}
    dtPolyRef path[4096]{};int count=0;status=query.findPath(refs[0],refs[1],near[0],near[1],&filter,path,&count,4096);out.push_back(status);out.push_back(count);for(int i=0;i<count;++i)reference(out,path[i]);
    float straight[4096*3]{};unsigned char flags[4096]{};dtPolyRef srefs[4096]{};int n=0;status=query.findStraightPath(near[0],near[1],path,count,straight,flags,srefs,&n,4096);out.push_back(status);out.push_back(n);for(int i=0;i<n;++i){point(out,straight+i*3);out.push_back(flags[i]);reference(out,srefs[i]);}
    return out;
}
int main(int argc,char** argv){try{
    if(argc!=5)return 2;Input in(argv[1]);if(in.word()!=0x31544444)return 3;
    int nv=in.word(),np=in.word();(void)in.word();int nvp=in.word();(void)in.word();(void)in.word();float meshMin[3],meshMax[3];for(float& f:meshMin)f=in.number();for(float& f:meshMax)f=in.number();float cs=in.number(),ch=in.number();
    if(nv<0||nv>65534||np<0||np>32768||(nvp<3&&np)||nvp>6)return 4;
    std::vector<unsigned short> verts(nv*3),polys(np*nvp*2),polyFlags(np);std::vector<unsigned char> areas(np);
    for(auto& v:verts)v=in.word();for(int i=0;i<np;++i){for(int j=0;j<nvp*2;++j)polys[i*nvp*2+j]=in.word();(void)in.word();areas[i]=in.word();polyFlags[i]=in.word();}
    unsigned nm=in.word(),dv=in.word(),dt=in.word();if(nm!=unsigned(np)||dv>unsigned(np)*127||dt>unsigned(np)*255)return 5;std::vector<unsigned> sub(nm*4);std::vector<float> detailVerts(dv*3);std::vector<unsigned char> tris(dt*4);for(auto& v:sub)v=in.word();for(auto& v:detailVerts)v=in.number();for(auto& v:tris)v=in.word();
    dtNavMeshCreateParams p{};p.verts=verts.data();p.vertCount=nv;p.polys=polys.data();p.polyCount=np;p.nvp=nvp;p.polyFlags=polyFlags.data();p.polyAreas=areas.data();p.detailMeshes=sub.data();p.detailVerts=detailVerts.data();p.detailVertsCount=dv;p.detailTris=tris.data();p.detailTriCount=dt;
    p.tileX=int(in.word());p.tileY=int(in.word());p.tileLayer=int(in.word());p.userId=in.word();for(float& f:p.bmin)f=in.number();for(float& f:p.bmax)f=in.number();p.walkableHeight=in.number();p.walkableRadius=in.number();p.walkableClimb=in.number();p.cs=cs;p.ch=ch;p.buildBvTree=in.word();auto flagPolicy=in.word();p.offMeshConCount=in.word();if(flagPolicy>1||p.offMeshConCount<0||p.offMeshConCount>65536)return 6;
    if(flagPolicy)for(int i=0;i<np;++i){auto area=areas[i]&63;if(area>11)return 7;if(area)polyFlags[i]=area>=8?1<<(11-area):1;}
    std::vector<float> ends(p.offMeshConCount*6),radii(p.offMeshConCount);std::vector<unsigned short> flags(p.offMeshConCount);std::vector<unsigned char> conAreas(p.offMeshConCount),directions(p.offMeshConCount);
    for(int i=0;i<p.offMeshConCount;++i){for(int k=0;k<6;++k)ends[i*6+k]=in.number();radii[i]=in.number();directions[i]=in.word();conAreas[i]=in.word();flags[i]=in.word();}
    unsigned probe=in.word();if(probe>2||in.f.peek()!=std::char_traits<char>::eof())return 8;p.offMeshConVerts=ends.data();p.offMeshConRad=radii.data();p.offMeshConDir=directions.data();p.offMeshConAreas=conAreas.data();p.offMeshConFlags=flags.data();
    unsigned char* data=nullptr;int bytes=0;bool ok=dtCreateNavMeshData(&p,&data,&bytes);if(!ok){if(nv&&np)return 9;save(argv[2],"",0);uint32_t rejected=0;save(argv[3],&rejected,4);return 0;}
    std::unique_ptr<unsigned char,decltype(&dtFree)> owned(data,&dtFree);save(argv[2],data,bytes);
    std::ifstream candidate(argv[4],std::ios::binary);std::vector<unsigned char> rust((std::istreambuf_iterator<char>(candidate)),{});if(rust.size()!=size_t(bytes))return 10;
    auto a=inspect(data,bytes,probe),b=inspect(rust.data(),bytes,probe);if(a!=b)return 11;
    if(a.empty()||dtStatusFailed(a[0]))return 12;save(argv[3],a.data(),a.size()*4);return 0;
}catch(std::exception const&){return 13;}}
