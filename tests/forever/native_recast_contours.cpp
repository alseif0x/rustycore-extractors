// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original complete supplied-field contour pipeline.
#define main unused_heightfield_driver
#include "native_recast_heightfield.cpp"
#undef main
#include <cstdio>
#include <cstring>
class ContourContext: public rcContext {
public: ContourContext():rcContext(true){} bool failed=false;
protected: void doLog(rcLogCategory category,char const* message,int) override {
    if(std::strstr(message,"rcBuildRegions:")&&std::strstr(message,"overlapping regions."))return;
    if(std::strstr(message,"rcBuildContours: Expanding max contours"))return;
    if(category==RC_LOG_WARNING||category==RC_LOG_ERROR)failed=true;
}};
int main(int argc,char** argv){try{
    if(argc!=3)return 2;Input in(argv[1]);Output out(argv[2]);if(in.word()!=0x31544352)return 3;
    int w=in.word(),h=in.word();if(w<1||w>1024||h<1||h>1024)return 4;
    float lo[3],hi[3];for(float& n:lo)n=in.number();for(float& n:hi)n=in.number();float cs=in.number(),ch=in.number();int height=in.word(),climb=in.word();auto ns=in.word();if(ns>2000000)return 5;
    ContourContext ctx;rcHeightfield field;rcCompactHeightfield compact;rcContourSet contours;
    if(!rcCreateHeightfield(&ctx,field,w,h,lo,hi,cs,ch))return 6;
    for(uint32_t i=0;i<ns;++i){auto x=in.word(),z=in.word(),lo=in.word(),hi=in.word(),area=in.word();if(x>=unsigned(w)||z>=unsigned(h)||lo>=hi||hi>65535||area>63)return 7;if(!rcAddSpan(&ctx,field,x,z,lo,hi,area,0))return 8;}
    auto radius=in.word(),median=in.word(),border=in.word(),minimum=in.word(),merge=in.word();float error=in.number();auto edge=in.word(),flags=in.word(),singleRegion=in.word();
    if(in.f.peek()!=std::char_traits<char>::eof()||median>1||border*2>unsigned(w)||border*2>unsigned(h)||minimum>0x7fffffff||merge>0x7fffffff||edge>46340||flags>3||singleRegion>1||(singleRegion&&border))return 9;
    if(!rcBuildCompactHeightfield(&ctx,height,climb,field,compact))return 10;
    if(radius!=0xffffffff){if(radius>127||!rcErodeWalkableArea(&ctx,radius,compact))return 11;}
    if(median&&!rcMedianFilterWalkableArea(&ctx,compact))return 12;
    if(!rcBuildDistanceField(&ctx,compact)||!rcBuildRegions(&ctx,compact,border,minimum,merge))return 13;
    // Explicit synthetic contour-only cases, separate from generated watershed labels.
    if(singleRegion){for(int i=0;i<compact.spanCount;++i)if(compact.spans[i].reg)compact.spans[i].reg=1;compact.maxRegions=1;}
    if(!rcBuildContours(&ctx,compact,error,edge,contours,flags)||ctx.failed)return 14;
    out.compact(compact);out.word(compact.maxRegions);out.word(compact.maxDistance);
    for(float n:contours.bmin)out.number(n);for(float n:contours.bmax)out.number(n);out.number(contours.cs);out.number(contours.ch);out.word(contours.width);out.word(contours.height);out.word(contours.borderSize);out.number(contours.maxError);out.word(contours.nconts);
    for(int i=0;i<contours.nconts;++i){auto const& c=contours.conts[i];out.word(c.reg);out.word(c.area);out.word(c.nverts);out.word(c.nrverts);for(int j=0;j<c.nverts*4;++j)out.word(c.verts[j]);for(int j=0;j<c.nrverts*4;++j)out.word(c.rverts[j]);}
    out.f.flush();return out.f?0:15;
}catch(std::exception const&){return 16;}}
