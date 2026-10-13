// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: actual public loadMap ancestor/neighborhood dispatch QA.
// Trusted complete synthetic positives; original reader/manager/math unchanged.
#define main supplied_terrain_navigation_qa_main
#include "native_navigation_terrain.cpp"
#undef main
#include "VMapManager.h"
#include <unordered_map>
int main(int argc,char**argv)try {
    if(argc!=3)throw std::runtime_error("usage");auto root=std::filesystem::canonical(argv[1]);std::ifstream plan(root/"plan.txt");
    unsigned np,map,row,col,skip,build;if(!(plan>>np>>map>>row>>col>>skip>>build)||np>64||row>=64||col>=64||skip>1)throw std::runtime_error("plan");
    std::unordered_map<uint32,std::vector<uint32>> topology;
    for(unsigned i=0;i<np;++i){uint32 id,p,c;if(!(plan>>id>>p>>c))throw std::runtime_error("parent");topology[id];if(p==UINT32_MAX)p=c;if(p!=UINT32_MAX)topology[p].push_back(id);}
    std::string tail;if(plan>>tail)throw std::runtime_error("tail");VMAP::VMapManager manager;manager.InitializeThreadUnsafe(topology);
    MMAP::TerrainBuilder builder(boost::filesystem::path(root.string()),skip!=0);MMAP::MeshData data;builder.loadMap(map,row,col,data,&manager);
    std::FILE* out=std::fopen(argv[2],"wbx");if(!out)throw std::runtime_error("output");values(out,data.solidVerts);values(out,data.solidTris);values(out,data.liquidVerts);values(out,data.liquidTris);values(out,data.liquidType);if(std::fclose(out))throw std::runtime_error("close");std::cout<<"{\"ok\":true}\n";return 0;
}catch(std::exception const&){std::cerr<<"{\"error\":\"native_navigation_terrain_load_qa\"}\n";return 4;}
