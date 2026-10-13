// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original public polygon builder on synthetic contour sets.
#define main unused_heightfield_driver
#include "native_recast_heightfield.cpp"
#undef main
#include <cstring>
#include "RecastAlloc.h"
class MeshContext: public rcContext {
public: MeshContext():rcContext(true){} bool failed=false; unsigned partial=0;
protected: void doLog(rcLogCategory category,char const* message,int) override {
    if(std::strstr(message,"rcBuildPolyMesh: Bad triangulation")||std::strstr(message,"removeVertex: triangulate() returned bad results")){++partial;return;}
    if(category==RC_LOG_WARNING||category==RC_LOG_ERROR)failed=true;
}};
int main(int argc,char** argv){try{
    if(argc!=3)return 2;Input in(argv[1]);Output out(argv[2]);if(in.word()!=0x31504d52)return 3;
    int nvp=in.word();rcContourSet contours;
    contours.width=in.word();contours.height=in.word();contours.borderSize=in.word();contours.maxError=in.number();
    for(float& n:contours.bmin)n=in.number();for(float& n:contours.bmax)n=in.number();contours.cs=in.number();contours.ch=in.number();
    contours.nconts=in.word();
    if(nvp<3||nvp>6||contours.width<0||contours.width>1024||contours.height<0||contours.height>1024||contours.nconts<0||contours.nconts>65533)return 4;
    contours.conts=(rcContour*)rcAlloc(sizeof(rcContour)*contours.nconts,RC_ALLOC_PERM);
    if(contours.nconts&&!contours.conts)return 5;
    if(contours.nconts)std::memset(contours.conts,0,sizeof(rcContour)*contours.nconts);
    for(int i=0;i<contours.nconts;++i){auto& c=contours.conts[i];auto reg=in.word(),area=in.word();c.nverts=in.word();c.nrverts=in.word();
        if(reg>65535||area>63||c.nverts<0||c.nverts>65533||c.nrverts<0||c.nrverts>65533)return 6;c.reg=reg;c.area=area;
        c.verts=(int*)rcAlloc(sizeof(int)*c.nverts*4,RC_ALLOC_PERM);c.rverts=(int*)rcAlloc(sizeof(int)*c.nrverts*4,RC_ALLOC_PERM);
        if((c.nverts&&!c.verts)||(c.nrverts&&!c.rverts))return 7;
        for(int j=0;j<c.nverts*4;++j)c.verts[j]=(int)in.word();for(int j=0;j<c.nrverts*4;++j)c.rverts[j]=(int)in.word();
    }
    if(in.f.peek()!=std::char_traits<char>::eof())return 8;
    MeshContext ctx;rcPolyMesh mesh;if(!rcBuildPolyMesh(&ctx,contours,nvp,mesh)||ctx.failed)return 9;
    out.word(ctx.partial?2:1);out.word(mesh.nverts);out.word(mesh.npolys);out.word(mesh.maxpolys);out.word(mesh.nvp);out.word(mesh.borderSize);out.number(mesh.maxEdgeError);
    for(float n:mesh.bmin)out.number(n);for(float n:mesh.bmax)out.number(n);out.number(mesh.cs);out.number(mesh.ch);
    for(int i=0;i<mesh.nverts*3;++i)out.word(mesh.verts[i]);
    for(int i=0;i<mesh.npolys;++i){for(int j=0;j<2*mesh.nvp;++j)out.word(mesh.polys[i*2*mesh.nvp+j]);out.word(mesh.regs[i]);out.word(mesh.areas[i]);out.word(mesh.flags[i]);}
    out.f.flush();return out.f?0:10;
}catch(std::exception const&){return 11;}}
