// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original fresh-navmesh addTile mutation and reload queries.
#define main detour_builder_main
#include "native_detour.cpp"
#undef main
int main(int argc,char** argv){try{
    if(argc!=5)return 2;
    std::ifstream in(argv[1],std::ios::binary);
    std::vector<unsigned char> bytes((std::istreambuf_iterator<char>(in)),{});
    if(bytes.size()<sizeof(dtMeshHeader))return 3;
    auto h=reinterpret_cast<dtMeshHeader*>(bytes.data());
    dtNavMeshParams params{};std::memcpy(params.orig,h->bmin,sizeof(params.orig));
    params.tileWidth=params.tileHeight=533.3333f;params.maxTiles=16;params.maxPolys=INT32_MIN;
    dtNavMesh nav;auto status=nav.init(&params);if(dtStatusFailed(status))return 4;
    dtTileRef ref=0;status=nav.addTile(bytes.data(),int(bytes.size()),0,0,&ref);
    if(dtStatusFailed(status)||ref!=(dtTileRef(1)<<52))return 5;
    save(argv[2],bytes.data(),bytes.size());
    auto runtime=inspect(bytes.data(),int(bytes.size()),unsigned(std::stoi(argv[4])));
    save(argv[3],runtime.data(),runtime.size()*4);
    nav.removeTile(ref,nullptr,nullptr);return 0;
}catch(std::exception const&){return 6;}}
