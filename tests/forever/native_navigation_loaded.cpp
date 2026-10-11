// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original loaded-model slot order and loadVMap lifecycle QA.
// Trusted complete synthetic packages only; original algorithms stay unchanged.
#define main supplied_navigation_model_qa_main
#include "native_navigation_models.cpp"
#undef main
#include <unordered_map>
#include <vector>
int main(int argc,char**argv)try {
    if(argc!=3)throw std::runtime_error("usage");auto root=std::filesystem::canonical(argv[1]);auto out=std::filesystem::path(argv[2]);std::filesystem::create_directory(out);
    std::ifstream plan(root/"plan.txt");unsigned np,no,nt;if(!(plan>>np>>no>>nt)||np>64||no>4096||nt>65536)throw std::runtime_error("plan");
    std::unordered_map<uint32,std::vector<uint32>> topology;std::vector<uint32> ids;
    for(unsigned i=0;i<np;++i){uint32 id,p,c;if(!(plan>>id>>p>>c))throw std::runtime_error("parent");ids.push_back(id);topology[id];if(p==UINT32_MAX)p=c;if(p!=UINT32_MAX)topology[p].push_back(id);}
    types.clear();for(unsigned i=0;i<nt;++i){uint32 id,bank;if(!(plan>>id>>bank)||bank>=31||!types.emplace(id,bank).second)throw std::runtime_error("types");}
    VMAP::VMapManager manager;manager.InitializeThreadUnsafe(topology);manager.LoadPathOnlyModels=true;manager.GetLiquidFlagsPtr=&liquidFlags;
    MMAP::TerrainBuilder builder(boost::filesystem::path(root.string()),true);
    for(unsigned i=0;i<no;++i){unsigned kind,map,x,y;if(!(plan>>kind>>map>>x>>y)||x>=64||y>=64)throw std::runtime_error("operation");MMAP::MeshData data;
        if(kind==0){if(manager.loadMap((root/"vmaps").string(),map,x,y)!=VMAP::LoadResult::Success)throw std::runtime_error("load");
            for(auto const& instance:manager.getModelsOnMap(map))if(auto model=instance.getWorldModel()){
                auto pos=instance.iPos;pos.x-=32*MMAP::GRID_SIZE;pos.y-=32*MMAP::GRID_SIZE;builder.loadVMapModel(model,pos,instance.GetInvRot(),instance.iScale,data,&manager);
            }
        }else if(kind==1)manager.unloadMap(map,x,y);
        else if(kind==2)manager.unloadMap(map);
        else if(kind==6){if(!builder.loadVMap(map,x,y,data,&manager))throw std::runtime_error("loadVMap");}
        else throw std::runtime_error("kind");
        auto models=manager.getModelsOnMap(map);unsigned active=0;for(auto const& m:models)active+=m.getWorldModel()!=nullptr;
        auto p=out/(std::to_string(i)+".bin");std::FILE* f=std::fopen(p.c_str(),"wbx");if(!f)throw std::runtime_error("write");
        put(f,kind);put(f,map);put(f,models.size());put(f,active);values(f,data.solidVerts);values(f,data.solidTris);values(f,data.liquidVerts);values(f,data.liquidTris);values(f,data.liquidType);if(std::fclose(f))throw std::runtime_error("close");
    }
    std::string tail;if(plan>>tail)throw std::runtime_error("tail");for(auto id:ids)manager.unloadMap(id);std::cout<<"{\"ok\":true}\n";return 0;
}catch(std::exception const&){std::cerr<<"{\"error\":\"native_navigation_loaded_qa\"}\n";return 4;}
