// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic ABI/container comparison using original headers
// and Detour. Does not execute the original full MapBuilder/MMapManager.
#include "MMapDefines.h"
#include "DetourAlloc.h"
#include <bit>
#include <cassert>
#include <cstring>
#include <fstream>
#include <iterator>
#include <vector>
static_assert(std::endian::native == std::endian::little);
static_assert(sizeof(MmapNavMeshHeader)==40 && sizeof(MmapTileHeader)==20);
static_assert(sizeof(OffMeshData)==44 && sizeof(dtPolyRef)==8);
static_assert(offsetof(MmapNavMeshHeader,params)==8 && offsetof(MmapNavMeshHeader,offmeshConnectionCount)==36);
static_assert(offsetof(OffMeshData,From)==12 && offsetof(OffMeshData,To)==24 && offsetof(OffMeshData,Radius)==36);
static_assert(offsetof(OffMeshData,ConnectionFlags)==40 && offsetof(OffMeshData,AreaId)==41 && offsetof(OffMeshData,Flags)==42);
static_assert(MMAP_VERSION==16 && DT_NAVMESH_VERSION==7);
using Bytes=std::vector<unsigned char>;
Bytes read(char const* path) { std::ifstream f(path,std::ios::binary); assert(f); return Bytes(std::istreambuf_iterator<char>(f),{}); }
void write(char const* path,void const* data,size_t size) { std::ofstream f(path,std::ios::binary); assert(f); f.write(static_cast<char const*>(data),size); assert(f); }
uint32_t word(Bytes const& b,size_t p) { assert(p+4<=b.size()); uint32_t n; std::memcpy(&n,b.data()+p,4); return n; }
float num(Bytes const& b,size_t p) { return std::bit_cast<float>(word(b,p)); }
int main(int argc,char** argv) {
    assert(argc==7);auto in=read(argv[1]); auto expectedGlobal=read(argv[2]); assert(in.size()>=40);
    uint32_t map=word(in,0), count=word(in,32); MmapNavMeshHeader h;
    for(size_t a=0;a<3;++a) h.params.orig[a]=num(in,4+a*4);
    h.params.tileWidth=num(in,16);h.params.tileHeight=num(in,20);h.params.maxTiles=std::bit_cast<int32_t>(word(in,24));h.params.maxPolys=std::bit_cast<int32_t>(word(in,28));
    std::vector<OffMeshData> selected;
    for(size_t i=0;i<count;++i) {
        size_t p=40+i*44;OffMeshData s{};s.MapId=word(in,p);s.TileX=word(in,p+4);s.TileY=word(in,p+8);
        for(size_t a=0;a<3;++a){s.From[a]=num(in,p+12+a*4);s.To[a]=num(in,p+24+a*4);}
        s.Radius=num(in,p+36);s.ConnectionFlags=static_cast<OffMeshConnectionFlag>(in[p+40]);s.AreaId=in[p+41];uint16_t flags;std::memcpy(&flags,in.data()+p+42,2);s.Flags=static_cast<NavTerrainFlag>(flags);
        if(s.MapId==map) selected.push_back(s);
    }
    h.offmeshConnectionCount=selected.size();Bytes global(sizeof(h)+sizeof(OffMeshData)*selected.size());std::memcpy(global.data(),&h,sizeof(h));if(!selected.empty())std::memcpy(global.data()+sizeof(h),selected.data(),sizeof(OffMeshData)*selected.size());assert(global==expectedGlobal);write(argv[4],global.data(),global.size());
    // Read the emitted global using the original native ABI, then initialize
    // original Detour with those actual decoded parameters.
    MmapNavMeshHeader readHeader;std::memcpy(&readHeader,expectedGlobal.data(),sizeof(readHeader));assert(readHeader.mmapMagic==MMAP_MAGIC && readHeader.mmapVersion==MMAP_VERSION);assert(readHeader.offmeshConnectionCount==selected.size());
    dtNavMesh mesh;dtStatus init=mesh.init(&readHeader.params);assert(dtStatusSucceed(init));
    size_t pos=40+count*44;uint32_t size=word(in,pos);assert(in.size()==pos+4+size);uint32_t result[4]={init,size,0,0};
    if(size) {
        MmapTileHeader th;th.size=size;th.usesLiquids=word(in,36)!=0;Bytes tile(sizeof(th)+size);std::memcpy(tile.data(),&th,sizeof(th));std::memcpy(tile.data()+sizeof(th),in.data()+pos+4,size);assert(tile==read(argv[3]));write(argv[5],tile.data(),tile.size());
        MmapTileHeader readTile;std::memcpy(&readTile,tile.data(),sizeof(readTile));assert(readTile.mmapMagic==MMAP_MAGIC && readTile.mmapVersion==MMAP_VERSION && readTile.dtVersion==DT_NAVMESH_VERSION);
        auto* data=static_cast<unsigned char*>(dtAlloc(readTile.size,DT_ALLOC_PERM));assert(data);std::memcpy(data,tile.data()+sizeof(readTile),readTile.size);dtTileRef ref=0;result[2]=mesh.addTile(data,readTile.size,DT_TILE_FREE_DATA,0,&ref);assert(dtStatusSucceed(result[2]) && ref);result[3]=mesh.removeTile(ref,nullptr,nullptr);assert(dtStatusSucceed(result[3]));
    }
    write(argv[6],result,sizeof(result));
}
