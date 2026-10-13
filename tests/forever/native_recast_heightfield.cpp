// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only driver of the unchanged pinned Recast API.
// The original Zlib implementation and its notices are retained externally.
#include "Recast.h"
#include <array>
#include <bit>
#include <cstdint>
#include <fstream>
#include <stdexcept>
#include <vector>

struct Input {
    std::ifstream f;
    explicit Input(char const* p):f(p,std::ios::binary) { if(!f) throw std::runtime_error("input"); }
    uint32_t word(){ unsigned char b[4]; if(!f.read(reinterpret_cast<char*>(b),4)) throw std::runtime_error("short input"); return b[0]|uint32_t(b[1])<<8|uint32_t(b[2])<<16|uint32_t(b[3])<<24; }
    float number(){return std::bit_cast<float>(word());}
};
struct Output {
    std::ofstream f;
    explicit Output(char const* p):f(p,std::ios::binary) { if(!f) throw std::runtime_error("output"); }
    void word(uint32_t n){for(int i=0;i<4;++i)f.put(char(n>>(i*8)));}
    void number(float n){word(std::bit_cast<uint32_t>(n));}
    void field(rcHeightfield const& h){for(int i=0;i<h.width*h.height;++i){uint32_t count=0;for(auto s=h.spans[i];s;s=s->next)++count;word(count);for(auto s=h.spans[i];s;s=s->next){word(s->smin);word(s->smax);word(s->area);}}}
    void compact(rcCompactHeightfield const& h){
        for(float n:h.bmin)number(n);for(float n:h.bmax)number(n);number(h.cs);number(h.ch);
        word(h.walkableHeight);word(h.walkableClimb);word(h.spanCount);
        for(int i=0;i<h.width*h.height;++i){word(h.cells[i].index);word(h.cells[i].count);}
        for(int i=0;i<h.spanCount;++i){auto const& s=h.spans[i];word(s.y);word(s.reg);for(int d=0;d<4;++d)word(rcGetCon(s,d));word(s.h);word(h.areas[i]);}
    }
};
struct Mesh {
    std::vector<float> vertices;std::vector<int> triangles;std::vector<unsigned char> areas;
    explicit Mesh(Input& in){auto nv=in.word(),nt=in.word();if(nv>2000000||nt>1000000)throw std::runtime_error("mesh limit");vertices.resize(nv*3);triangles.resize(nt*3);areas.resize(nt);for(auto& v:vertices)v=in.number();for(auto& t:triangles){auto n=in.word();if(n>=nv)throw std::runtime_error("index");t=int(n);}for(auto& a:areas){auto n=in.word();if(n>63)throw std::runtime_error("area");a=n;}}
    void raster(rcContext& ctx,rcHeightfield& h,int merge){if(!rcRasterizeTriangles(&ctx,vertices.data(),int(vertices.size()/3),triangles.data(),areas.data(),int(areas.size()),h,merge))throw std::runtime_error("raster");}
};
int main(int argc,char** argv){try{
    if(argc!=3)return 2;Input in(argv[1]);Output out(argv[2]);if(in.word()!=0x31524352)return 3;
    int w=in.word(),h=in.word();if(w<1||w>1024||h<1||h>1024)return 4;
    float lo[3],hi[3];for(float& n:lo)n=in.number();for(float& n:hi)n=in.number();float cs=in.number(),ch=in.number(),maximum=in.number(),nonsteep=in.number();
    int height=in.word(),climb=in.word(),merge=in.word();unsigned filters=in.word();auto ns=in.word();if(ns>2000000)return 5;
    std::vector<std::array<uint32_t,5>> spans(ns);for(auto& s:spans)for(auto& n:s)n=in.word();
    Mesh solid(in),liquid(in);if(in.f.peek()!=std::char_traits<char>::eof())return 6;
    rcContext ctx(false);rcHeightfield field;rcCompactHeightfield compact;
    if(!rcCreateHeightfield(&ctx,field,w,h,lo,hi,cs,ch))return 7;
    for(auto s:spans){if(s[0]>=unsigned(w)||s[1]>=unsigned(h)||s[2]>=s[3]||s[3]>65535||s[4]>63)return 8;if(!rcAddSpan(&ctx,field,s[0],s[1],s[2],s[3],s[4],merge))return 9;}
    std::fill(solid.areas.begin(),solid.areas.end(),10);
    rcClearUnwalkableTriangles(&ctx,maximum,solid.vertices.data(),solid.vertices.size()/3,solid.triangles.data(),solid.areas.size(),solid.areas.data());
    rcMarkWalkableTriangles(&ctx,nonsteep,solid.vertices.data(),solid.vertices.size()/3,solid.triangles.data(),solid.areas.size(),solid.areas.data(),11);
    out.word(solid.areas.size());for(auto a:solid.areas)out.word(a);out.field(field);
    solid.raster(ctx,field,merge);out.field(field);
    if(filters&1)rcFilterLowHangingWalkableObstacles(&ctx,climb,field);out.field(field);
    if(filters&2)rcFilterLedgeSpans(&ctx,height,climb,field);out.field(field);
    if(filters&4)rcFilterWalkableLowHeightSpans(&ctx,height,field);out.field(field);
    liquid.raster(ctx,field,merge);out.field(field);
    if(!rcBuildCompactHeightfield(&ctx,height,climb,field,compact))return 10;
    out.compact(compact);out.f.flush();if(!out.f)return 11;return 0;
}catch(std::exception const&){return 12;}}
