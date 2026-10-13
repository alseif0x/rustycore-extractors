// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original DB2 numeric access and complete MapBuilder
// discovery/filter/root-navmesh QA. Trusted bounded synthetic packages only.
// Original implementation units are included unchanged; -fno-access-control
// exposes private inventory for inspection without replacing its algorithms.
#define main gameobject_source_qa_main
#include "native_gameobject_sources.cpp"
#undef main
#include "MapBuilder.cpp"
#include <fstream>
namespace MMAP {std::unordered_map<uint32,MapEntry> sMapStore;}
void write_words(std::filesystem::path const& path,std::vector<uint32> const& words){
    std::ofstream f(path,std::ios::binary);f.write(reinterpret_cast<char const*>(words.data()),words.size()*4);if(!f)throw Failure{"write"};
}
int main(int argc,char**argv)try{
    if(argc!=3)return 2;auto root=std::filesystem::canonical(argv[1]);auto out=std::filesystem::path(argv[2]);std::filesystem::create_directories(out/"mmaps");
    Input input(root/"Map.db2");auto const& bytes=input.bytes;if(bytes.size()<204||word(bytes,140)!=26||word(bytes,156)!=0xd43afac3||word(bytes,136)>200000)return 3;
    std::vector<uint32> projection;
    {Silence quiet;MemorySource source(bytes);DB2FileLoader loader;loader.Load(&source,&MapLoadInfo::Instance);auto nr=loader.GetRecordCount(),nc=loader.GetRecordCopyCount();projection={nr,nc};
    for(uint32 i=0;i<nr;++i){auto r=loader.GetRecord(i);if(!r)return 4;int16 parent=int16(r.GetUInt16("ParentMapID")),cosmetic=int16(r.GetUInt16("CosmeticParentMapID"));
        auto& m=MMAP::sMapStore[r.GetId()];m.MapType=r.GetUInt8("MapType");m.InstanceType=r.GetUInt8("InstanceType");m.ParentMapID=parent<0?cosmetic:parent;m.Flags=r.GetInt32("Flags1");
        projection.insert(projection.end(),{r.GetId(),uint32(m.MapType),uint32(int32(m.InstanceType)),uint32(int32(parent)),uint32(int32(cosmetic)),uint32(m.Flags)});
    }
    for(uint32 i=0;i<nc;++i){auto c=loader.GetRecordCopy(i);projection.insert(projection.end(),{c.NewRowId,c.SourceRowId});}quiet.finish();}
    input.verify();write_words(out/"source.bin",projection);
    Input options(root/"options.bin");if(options.bytes.size()!=16)return 5;bool selected=word(options.bytes,0);uint32 map=word(options.bytes,4),flags=word(options.bytes,8);if(selected&&map>INT32_MAX)return 6;
    Silence quiet;
    MMAP::MapBuilder builder(root.string(),out.string(),{}, {},false,flags&1,flags&2,flags&4,false,false,selected?int(map):-1,nullptr,1);
    std::vector<uint32> ids;for(auto const& [id,_]:builder.m_tiles)ids.push_back(id);std::sort(ids.begin(),ids.end());std::vector<uint32> inventory{uint32(ids.size()),builder.m_totalTiles};
    for(auto id:ids){auto tiles=builder.getTileList(id);bool produce=!tiles.empty()&&(!selected||map==id);inventory.insert(inventory.end(),{id,uint32(tiles.size()),uint32(produce)});inventory.insert(inventory.end(),tiles.begin(),tiles.end());
        if(produce){dtNavMesh* nav=nullptr;builder.buildNavMesh(id,nav);if(!nav)return 7;
            auto p=nav->getParams();std::vector<uint32> loc{uint32(nav->getMaxTiles()),25};
            for(int x=-2;x<=2;++x)for(int z=-2;z<=2;++z){float point[3]={p->orig[0]+p->tileWidth*(x+0.25f),p->orig[1],p->orig[2]+p->tileHeight*(z+0.75f)};int tx,tz;nav->calcTileLoc(point,&tx,&tz);for(float f:point)loc.push_back(std::bit_cast<uint32>(f));loc.insert(loc.end(),{uint32(tx),uint32(tz)});}
            write_words(out/(std::to_string(id)+".locations"),loc);dtFreeNavMesh(nav);}
    }
    quiet.finish();write_words(out/"inventory.bin",inventory);options.verify();return 0;
}catch(...){return 8;}
