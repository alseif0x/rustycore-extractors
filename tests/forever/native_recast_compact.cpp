// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic original Recast area/distance caller.
// Reuse only the authored checked wire helpers; original sources stay external.
#define main unused_heightfield_driver
#include "native_recast_heightfield.cpp"
#undef main

int main(int argc,char** argv){try{
    if(argc!=3)return 2;Input in(argv[1]);Output out(argv[2]);if(in.word()!=0x31434352)return 3;
    int w=in.word(),h=in.word();if(w<1||w>1024||h<1||h>1024)return 4;
    float lo[3],hi[3];for(float& n:lo)n=in.number();for(float& n:hi)n=in.number();float cs=in.number(),ch=in.number();
    int height=in.word(),climb=in.word();auto ns=in.word();if(ns>2000000)return 5;
    rcContext ctx(false);rcHeightfield field;rcCompactHeightfield compact;
    if(!rcCreateHeightfield(&ctx,field,w,h,lo,hi,cs,ch))return 6;
    for(uint32_t i=0;i<ns;++i){auto x=in.word(),z=in.word(),lo=in.word(),hi=in.word(),area=in.word();if(x>=unsigned(w)||z>=unsigned(h)||lo>=hi||hi>65535||area>63)return 7;if(!rcAddSpan(&ctx,field,x,z,lo,hi,area,0))return 8;}
    if(!rcBuildCompactHeightfield(&ctx,height,climb,field,compact))return 9;
    out.compact(compact);auto ne=in.word();if(ne>4)return 10;
    for(uint32_t i=0;i<ne;++i){auto radius=in.word();if(radius>127)return 11;if(!rcErodeWalkableArea(&ctx,radius,compact))return 12;out.compact(compact);}
    auto nm=in.word();if(nm>4)return 13;
    for(uint32_t i=0;i<nm;++i){if(!rcMedianFilterWalkableArea(&ctx,compact))return 14;out.compact(compact);}
    if(in.f.peek()!=std::char_traits<char>::eof())return 15;
    if(!rcBuildDistanceField(&ctx,compact))return 16;
    out.compact(compact);out.word(compact.maxDistance);out.word(compact.spanCount);for(int i=0;i<compact.spanCount;++i)out.word(compact.dist[i]);
    out.f.flush();return out.f?0:17;
}catch(std::exception const&){return 18;}}
