// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: actual public original TileBuilder::buildTile pipeline QA.
// Complete authored synthetic packages only; bounded explicit manager factory.
#include "TileBuilder.h"
#include "VMapManager.h"
#include <filesystem>
#include <unordered_map>
#include <iostream>
#define main unused_navigation_detour_main
#include "native_detour.cpp"
#undef main
static std::unordered_map<uint32,std::vector<uint32>> topology;
static std::unordered_map<uint32,uint8> liquidBanks;
// The base TileBuilder skips all non-debug tiles. The production worker supplies
// its existing-output policy through this virtual gate. QA selects exactly one
// fresh tile; every original source/build/save implementation remains unchanged.
struct SelectedTileBuilder : MMAP::TileBuilder {
    using MMAP::TileBuilder::TileBuilder;
    bool shouldSkipTile(uint32,uint32,uint32) const override {return false;}
};
static std::unique_ptr<VMAP::VMapManager> manager(uint32) {
    auto m=std::make_unique<VMAP::VMapManager>();m->InitializeThreadUnsafe(topology);
    m->LoadPathOnlyModels=true;m->GetLiquidFlagsPtr=[](uint32 id)->uint32 {auto p=liquidBanks.find(id);return p==liquidBanks.end()?0:(1<<p->second);};return m;
}
int main(int argc,char**argv)try {
    if(argc!=4)throw std::runtime_error("usage");auto root=std::filesystem::canonical(argv[1]);
    std::ifstream in(root/"plan.txt");uint32 np,map,row,col,skip,build,big,nt,nc;float slope,non;
    if(!(in>>np>>map>>row>>col>>skip>>build>>big>>slope>>non>>nt>>nc)||np>64||row>=64||col>=64||skip>1||big>1||nt>65536||nc>65536)throw std::runtime_error("plan");
    for(uint32 i=0;i<np;++i){uint32 id,p,c,bits;if(!(in>>id>>p>>c>>bits))throw std::runtime_error("parent");topology[id];if(p==UINT32_MAX)p=c;if(p!=UINT32_MAX)topology[p].push_back(id);}
    for(uint32 i=0;i<nt;++i){uint32 id,bank;if(!(in>>id>>bank)||bank>255)throw std::runtime_error("liquid");liquidBanks[id]=bank;}
    for(auto const& [id,bank]:liquidBanks)if(bank>=31)throw std::runtime_error("final liquid bank");
    std::vector<OffMeshData> connections;
    for(uint32 i=0;i<nc;++i){OffMeshData s{};uint32 d,a,f;
        if(!(in>>s.MapId>>s.TileX>>s.TileY))throw std::runtime_error("connection");
        for(float&v:s.From)if(!(in>>v))throw std::runtime_error("from");for(float&v:s.To)if(!(in>>v))throw std::runtime_error("to");
        if(!(in>>s.Radius>>d>>a>>f))throw std::runtime_error("flags");s.ConnectionFlags=OffMeshConnectionFlag(d);s.AreaId=a;s.Flags=NavTerrainFlag(f);connections.push_back(s);}
    std::string tail;if(in>>tail)throw std::runtime_error("tail");
    Input global(argv[2]);if(global.word()!=0x4d4d4150||global.word()!=16)throw std::runtime_error("global");dtNavMeshParams params{};
    for(float&v:params.orig)v=global.number();params.tileWidth=global.number();params.tileHeight=global.number();params.maxTiles=global.word();params.maxPolys=global.word();
    if(std::filesystem::exists(argv[3]))throw std::runtime_error("existing output");std::filesystem::create_directories(std::filesystem::path(argv[3])/"mmaps");
    MMAP::CreateVMapManager=&manager;dtNavMesh mesh;if(dtStatusFailed(mesh.init(&params)))throw std::runtime_error("navmesh");
    SelectedTileBuilder builder(root.string(),argv[3],slope,non,skip,big,false,&connections);builder.buildTile(map,row,col,&mesh);
    auto name=Trinity::StringFormat("{:04}_{:02}_{:02}.mmtile",map,row,col);auto file=std::filesystem::path(argv[3])/"mmaps"/name;
    if(std::filesystem::exists(file)){std::ifstream f(file,std::ios::binary);std::vector<unsigned char>b((std::istreambuf_iterator<char>(f)),{});if(b.size()<120)throw std::runtime_error("tile");auto w=inspect(b.data()+20,b.size()-20,1);auto out=std::filesystem::path(argv[3])/"runtime.bin";save(out.c_str(),w.data(),w.size()*4);}
    std::cout<<"{\"ok\":true}\n";return 0;
}catch(std::exception const&e){std::cerr<<e.what()<<"\n";return 4;}
