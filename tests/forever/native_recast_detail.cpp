// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original full synthetic supplied-span detail pipeline.
#define main unused_heightfield_driver
#include "native_recast_heightfield.cpp"
#undef main
#include <cstring>
#include <cstdio>
#include <memory>
class DetailContext: public rcContext {
public: DetailContext():rcContext(true){} bool failed=false;unsigned partial=0;
protected: void doLog(rcLogCategory category,char const* message,int length) override {
    if(std::strstr(message,"rcBuildRegions:")&&std::strstr(message,"overlapping regions."))return;
    if(std::strstr(message,"rcBuildContours: Expanding max contours"))return;
    if(std::strstr(message,"delaunayHull: Removing dangling face")){++partial;std::fprintf(stderr,"%.*s\n",length,message);return;}
    if(category==RC_LOG_WARNING||category==RC_LOG_ERROR){failed=true;std::fprintf(stderr,"%.*s\n",length,message);}
}};
int main(int argc,char** argv){try{
    if(argc!=3)return 2;Input in(argv[1]);Output out(argv[2]);if(in.word()!=0x31544452)return 3;
    int w=in.word(),h=in.word();if(w<1||w>1024||h<1||h>1024)return 4;
    float lo[3],hi[3];for(float& n:lo)n=in.number();for(float& n:hi)n=in.number();float cs=in.number(),ch=in.number();int height=in.word(),climb=in.word();auto ns=in.word();if(ns>2000000)return 5;
    DetailContext ctx;rcHeightfield field;rcCompactHeightfield compact;rcContourSet contours;rcPolyMesh mesh;
    std::unique_ptr<rcPolyMeshDetail,decltype(&rcFreePolyMeshDetail)> detailOwner(rcAllocPolyMeshDetail(),&rcFreePolyMeshDetail);
    if(!detailOwner)return 6;auto& detail=*detailOwner;
    if(!rcCreateHeightfield(&ctx,field,w,h,lo,hi,cs,ch))return 6;
    for(uint32_t i=0;i<ns;++i){auto x=in.word(),z=in.word(),lo=in.word(),hi=in.word(),area=in.word();if(x>=unsigned(w)||z>=unsigned(h)||lo>=hi||hi>65535||area>63)return 7;if(!rcAddSpan(&ctx,field,x,z,lo,hi,area,0))return 8;}
    auto radius=in.word(),median=in.word(),border=in.word(),minimum=in.word(),merge=in.word();float error=in.number();auto edge=in.word(),flags=in.word(),nvp=in.word(),regionMode=in.word();float distance=in.number(),detailError=in.number();
    if(in.f.peek()!=std::char_traits<char>::eof()||median>1||border*2>unsigned(w)||border*2>unsigned(h)||minimum>0x7fffffff||merge>0x7fffffff||edge>46340||flags>3||nvp<3||nvp>6||regionMode>2||!(distance>0))return 9;
    if(!rcBuildCompactHeightfield(&ctx,height,climb,field,compact))return 10;
    if(radius!=0xffffffff){if(radius>127||!rcErodeWalkableArea(&ctx,radius,compact))return 11;}
    if(median&&!rcMedianFilterWalkableArea(&ctx,compact))return 12;
    if(!rcBuildDistanceField(&ctx,compact)||!rcBuildRegions(&ctx,compact,border,minimum,merge))return 13;
    if(!rcBuildContours(&ctx,compact,error,edge,contours,flags)||!rcBuildPolyMesh(&ctx,contours,nvp,mesh)||ctx.failed)return 14;
    // Explicit synthetic metadata overrides exercise the two center-seeding paths.
    if(regionMode)for(int i=0;i<mesh.npolys;++i)mesh.regs[i]=regionMode==1?0:30000;
    if(!rcBuildPolyMeshDetail(&ctx,mesh,compact,distance,detailError,detail)||ctx.failed)return 15;
    out.word(1);out.compact(compact);out.word(compact.maxRegions);out.word(compact.maxDistance);
    out.word(mesh.nverts);out.word(mesh.npolys);out.word(mesh.maxpolys);out.word(mesh.nvp);out.word(mesh.borderSize);out.number(mesh.maxEdgeError);
    for(float n:mesh.bmin)out.number(n);for(float n:mesh.bmax)out.number(n);out.number(mesh.cs);out.number(mesh.ch);
    for(int i=0;i<mesh.nverts*3;++i)out.word(mesh.verts[i]);
    for(int i=0;i<mesh.npolys;++i){for(int j=0;j<2*mesh.nvp;++j)out.word(mesh.polys[i*2*mesh.nvp+j]);out.word(mesh.regs[i]);out.word(mesh.areas[i]);out.word(mesh.flags[i]);}
    out.word(detail.nmeshes);out.word(detail.nverts);out.word(detail.ntris);
    for(int i=0;i<detail.nmeshes*4;++i)out.word(detail.meshes[i]);
    for(int i=0;i<detail.nverts*3;++i)out.number(detail.verts[i]);
    for(int i=0;i<detail.ntris*4;++i)out.word(detail.tris[i]);
    out.word(ctx.partial);
    out.f.flush();return out.f?0:16;
}catch(std::exception const&){return 17;}}
