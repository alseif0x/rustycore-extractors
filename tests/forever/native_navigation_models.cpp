// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original supplied-model/offmesh navigation QA wrapper.
// Original WorldModel, ModelSpawn, ModelInstance and TerrainBuilder are unchanged.
// Only reviewed synthetic positives run; no map inventory/fallback is exercised.
#include "MMapDefines.h"
#include "WorldModel.h"
#include "ModelInstance.h"
#include "VMapManager.h"
#include <boost/filesystem/path.hpp>
#define private public
#include "TerrainBuilder.h"
#undef private
#include <array>
#include <bit>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <map>
#include <stdexcept>

std::map<uint32,uint8> types;
uint32 liquidFlags(uint32 id) {auto i=types.find(id);return i==types.end()?0:(uint32(1)<<i->second);}
uint32 word(std::istream& f) {std::array<unsigned char,4>b{};if(!f.read(reinterpret_cast<char*>(b.data()),4))throw std::runtime_error("read");return b[0]|uint32(b[1])<<8|uint32(b[2])<<16|uint32(b[3])<<24;}
void put(std::FILE* f,uint32 v){std::array<unsigned char,4>b{static_cast<unsigned char>(v),static_cast<unsigned char>(v>>8),static_cast<unsigned char>(v>>16),static_cast<unsigned char>(v>>24)};if(std::fwrite(b.data(),1,4,f)!=4)throw std::runtime_error("write");}
template<class T>void values(std::FILE* f,std::vector<T> const& a){put(f,a.size());for(T v:a){if constexpr(std::is_same_v<T,float>)put(f,std::bit_cast<uint32>(v));else if constexpr(sizeof(T)==4)put(f,static_cast<uint32>(v));else{std::array<unsigned char,sizeof(T)>b{};for(unsigned i=0;i<sizeof(T);++i)b[i]=static_cast<unsigned char>(v>>(i*8));if(std::fwrite(b.data(),1,b.size(),f)!=b.size())throw std::runtime_error("write");}}}
void file(std::filesystem::path const& p,std::uintmax_t cap) {if(!std::filesystem::is_regular_file(p)||std::filesystem::is_symlink(p)||std::filesystem::file_size(p)>cap)throw std::runtime_error("input");}
int main(int argc,char**argv)try{
    if(argc!=3)throw std::runtime_error("usage");auto root=std::filesystem::canonical(argv[1]);std::ifstream plan(root/"plan.txt");
    unsigned count,compact,prefix,map,row,col,typeCount,offCount;
    if(!(plan>>count>>compact>>prefix>>map>>row>>col>>typeCount>>offCount)||count>4||compact>1||prefix>1||row>=64||col>=64||typeCount>65536||offCount>65536)throw std::runtime_error("plan");
    for(unsigned i=0;i<typeCount;++i){unsigned id,bank;if(!(plan>>id>>bank)||bank>=31||!types.emplace(id,bank).second)throw std::runtime_error("type");}
    std::string tail;if(plan>>tail)throw std::runtime_error("tail");
    MMAP::TerrainBuilder builder(boost::filesystem::path(root.string()),true);MMAP::MeshData data;VMAP::VMapManager manager;manager.GetLiquidFlagsPtr=&liquidFlags;
    if(prefix){file(root/"maps/0000_24_39.map",202112);builder.loadMap(0,24,39,data,nullptr,MMAP::TOP);}
    for(unsigned i=0;i<count;++i){
        auto mp=root/("model-"+std::to_string(i)+".vmo");auto sp=root/("spawn-"+std::to_string(i)+".bin");file(mp,128*1024*1024);file(sp,1024);
        auto model=std::make_shared<VMAP::WorldModel>();if(!model->readFile(mp.string()))throw std::runtime_error("model");
        std::FILE* f=std::fopen(sp.c_str(),"rb");if(!f)throw std::runtime_error("spawn");VMAP::ModelSpawn spawn{};
        if(!VMAP::ModelSpawn::readFromFile(f,spawn)||std::fgetc(f)!=EOF){std::fclose(f);throw std::runtime_error("spawn");}std::fclose(f);
        VMAP::ModelInstance instance(spawn,model);auto position=instance.iPos;position.x-=32*MMAP::GRID_SIZE;position.y-=32*MMAP::GRID_SIZE;
        builder.loadVMapModel(model.get(),position,instance.GetInvRot(),instance.iScale,data,&manager);
    }
    std::vector<OffMeshData> connections;file(root/"connections.bin",65536*44);std::ifstream off(root/"connections.bin",std::ios::binary);
    for(unsigned i=0;i<offCount;++i){OffMeshData c{};c.MapId=word(off);c.TileX=word(off);c.TileY=word(off);for(auto&v:c.From)v=std::bit_cast<float>(word(off));for(auto&v:c.To)v=std::bit_cast<float>(word(off));c.Radius=std::bit_cast<float>(word(off));auto end=word(off);c.ConnectionFlags=static_cast<OffMeshConnectionFlag>(end&255);c.AreaId=(end>>8)&255;c.Flags=static_cast<NavTerrainFlag>(end>>16);connections.push_back(c);}
    if(off.peek()!=EOF)throw std::runtime_error("off_tail");builder.loadOffMeshConnections(map,row,col,data,connections);
    if(compact){builder.cleanVertices(data.solidVerts,data.solidTris);builder.cleanVertices(data.liquidVerts,data.liquidTris);}
    std::FILE* out=std::fopen(argv[2],"wbx");if(!out)throw std::runtime_error("output");
    values(out,data.solidVerts);values(out,data.solidTris);values(out,data.liquidVerts);values(out,data.liquidTris);values(out,data.liquidType);
    values(out,data.offMeshConnections);values(out,data.offMeshConnectionRads);values(out,data.offMeshConnectionDirs);values(out,data.offMeshConnectionsAreas);values(out,data.offMeshConnectionsFlags);
    if(std::fclose(out))throw std::runtime_error("close");std::cout<<"{\"ok\":true}\n";return 0;
}catch(std::exception const&){std::cerr<<"{\"error\":\"native_navigation_model_qa_failed\"}\n";return 4;}
