// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original complete supplied-geometry TileBuilder and config.
// Include the untouched original unit to expose its local TileConfig definition.
#include "TileBuilder.cpp"
#include <filesystem>
#define main unused_detour_corpus_main
#include "native_detour.cpp"
#undef main
int main(int argc,char** argv) {try {
    if(argc!=6)return 2;Input in(argv[1]);if(in.word()!=0x314c4254)return 3;
    uint32 map=in.word(),row=in.word(),col=in.word();bool big=in.word();float slope=in.number(),non=in.number();uint32 run=in.word();
    dtNavMeshParams params{};for(float&v:params.orig)v=in.number();params.tileWidth=in.number();params.tileHeight=in.number();params.maxTiles=in.word();params.maxPolys=in.word();
    uint32 sv=in.word(),st=in.word(),lv=in.word(),lt=in.word(),nc=in.word();if(sv>250000||lv>250000||st>500000||lt>500000||nc>65536)return 4;
    MMAP::MeshData m;m.solidVerts.resize(sv*3);m.solidTris.resize(st*3);m.liquidVerts.resize(lv*3);m.liquidTris.resize(lt*3);m.liquidType.resize(lt);
    for(float&v:m.solidVerts)v=in.number();for(int&v:m.solidTris)v=in.word();for(float&v:m.liquidVerts)v=in.number();for(int&v:m.liquidTris)v=in.word();for(auto&v:m.liquidType)v=in.word();
    for(uint32 i=0;i<nc;++i){for(int a=0;a<6;++a)m.offMeshConnections.push_back(in.number());m.offMeshConnectionRads.push_back(in.number());m.offMeshConnectionDirs.push_back(in.word());m.offMeshConnectionsAreas.push_back(in.word());m.offMeshConnectionsFlags.push_back(in.word());}
    if(in.f.peek()!=std::char_traits<char>::eof())return 5;
    std::string producerRoot=std::string(argv[2])+".producer";
    MMAP::TileBuilder builder(".",producerRoot,slope,non,false,big,false,nullptr);MMAP::TileConfig tile(big);std::vector<float> all=m.liquidVerts;all.insert(all.end(),m.solidVerts.begin(),m.solidVerts.end());float lo[3],hi[3];builder.getTileBounds(row,col,all.data(),all.size()/3,lo,hi);auto config=builder.GetMapSpecificConfig(map,lo,hi,tile);rcCalcGridSize(config.bmin,config.bmax,config.cs,&config.width,&config.height);
    std::vector<uint32_t> c;auto f=[&](float v){c.push_back(std::bit_cast<uint32_t>(v));};auto n=[&](int v){c.push_back(v);};
    f(tile.BASE_UNIT_DIM);f(config.cs);f(config.ch);n(config.tileSize);n(config.borderSize);n(config.walkableRadius);n(config.walkableHeight);n(config.walkableClimb);n(config.maxEdgeLen);n(config.minRegionArea);n(config.mergeRegionArea);n(config.maxVertsPerPoly);f(config.maxSimplificationError);f(config.detailSampleDist);f(config.detailSampleMaxError);f(config.walkableSlopeAngle);f(config.walkableSlopeAngleNotSteep);n(config.width);n(config.height);point(c,lo);point(c,hi);
    for(int y=0;y<tile.TILES_PER_MAP;++y)for(int x=0;x<tile.TILES_PER_MAP;++x){float bmin[3],bmax[3];rcVcopy(bmin,lo);rcVcopy(bmax,hi);bmin[0]=lo[0]+x*float(config.tileSize*config.cs);bmin[2]=lo[2]+y*float(config.tileSize*config.cs);bmax[0]=lo[0]+(x+1)*float(config.tileSize*config.cs);bmax[2]=lo[2]+(y+1)*float(config.tileSize*config.cs);bmin[0]-=config.borderSize*config.cs;bmin[2]-=config.borderSize*config.cs;bmax[0]+=config.borderSize*config.cs;bmax[2]+=config.borderSize*config.cs;point(c,bmin);point(c,bmax);n(config.tileSize+2*config.borderSize);}
    save(argv[2],c.data(),c.size()*4);if(!run)return 0;
    auto result=builder.buildMoveMapTile(map,row,col,m,lo,hi,&params);if(!result.data){save(argv[3],"",0);uint32_t zero=0;save(argv[4],&zero,4);return 0;}
    save(argv[3],result.data.get(),result.size);std::ifstream cf(argv[5],std::ios::binary);std::vector<unsigned char> rust((std::istreambuf_iterator<char>(cf)),{});if(rust.size()!=size_t(result.size))return 6;auto a=inspect(result.data.get(),result.size,1);auto b=inspect(rust.data(),rust.size(),1);if(a!=b)return 7;if(a.empty()||dtStatusFailed(a[0]))return 8;save(argv[4],a.data(),a.size()*4);
    // Also retain the real producer output. addTile mutates the buffer before
    // fwrite; this is separate evidence from the raw builder payload comparison.
    std::filesystem::create_directories(producerRoot+"/mmaps");dtNavMesh publishMesh;
    if(dtStatusFailed(publishMesh.init(&params)))return 10;
    builder.saveMoveMapTileToFile(map,row,col,&publishMesh,result);
    std::ifstream published(Trinity::StringFormat("{}/mmaps/{:04}_{:02}_{:02}.mmtile",producerRoot,map,row,col),std::ios::binary);
    if(!published)return 11;return 0;
}catch(std::exception const& e){std::fprintf(stderr,"%s\n",e.what());return 9;}}
