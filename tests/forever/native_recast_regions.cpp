// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original public watershed API with bounded synthetic input.
#define main unused_heightfield_driver
#include "native_recast_heightfield.cpp"
#undef main
#include <cstdio>
class RegionContext: public rcContext {
public:
    RegionContext():rcContext(true){}
    unsigned overlaps=0;bool unexpected=false;
protected:
    void doLog(rcLogCategory category,char const* message,int) override {
        unsigned count;
        if(std::sscanf(message,"rcBuildRegions: %u overlapping regions.",&count)==1)overlaps=count;
        else if(category==RC_LOG_ERROR)unexpected=true;
    }
};
int main(int argc,char** argv){try{
    if(argc!=3)return 2;Input in(argv[1]);Output out(argv[2]);if(in.word()!=0x31475252)return 3;
    int w=in.word(),h=in.word();if(w<1||w>1024||h<1||h>1024)return 4;
    float lo[3],hi[3];for(float& n:lo)n=in.number();for(float& n:hi)n=in.number();float cs=in.number(),ch=in.number();
    int height=in.word(),climb=in.word();auto ns=in.word();if(ns>2000000)return 5;
    RegionContext ctx;rcHeightfield field;rcCompactHeightfield compact;
    if(!rcCreateHeightfield(&ctx,field,w,h,lo,hi,cs,ch))return 6;
    for(uint32_t i=0;i<ns;++i){auto x=in.word(),z=in.word(),lo=in.word(),hi=in.word(),area=in.word();if(x>=unsigned(w)||z>=unsigned(h)||lo>=hi||hi>65535||area>63)return 7;if(!rcAddSpan(&ctx,field,x,z,lo,hi,area,0))return 8;}
    if(!rcBuildCompactHeightfield(&ctx,height,climb,field,compact))return 9;
    auto radius=in.word(),median=in.word(),border=in.word(),minimum=in.word(),merge=in.word();if(in.f.peek()!=std::char_traits<char>::eof())return 10;
    if(radius!=0xffffffff){if(radius>127)return 11;if(!rcErodeWalkableArea(&ctx,radius,compact))return 12;}
    if(median>1||border>65535||minimum>0x7fffffff||merge>0x7fffffff)return 13;
    if(median&&!rcMedianFilterWalkableArea(&ctx,compact))return 14;
    if(!rcBuildDistanceField(&ctx,compact))return 15;
    out.compact(compact);out.word(compact.maxDistance);out.word(compact.spanCount);for(int i=0;i<compact.spanCount;++i)out.word(compact.dist[i]);
    if(!rcBuildRegions(&ctx,compact,border,minimum,merge))return 16;
    out.compact(compact);out.word(compact.borderSize);out.word(compact.maxRegions);out.word(ctx.overlaps);
    if(ctx.unexpected)return 17;out.f.flush();return out.f?0:18;
}catch(std::exception const&){return 19;}}
