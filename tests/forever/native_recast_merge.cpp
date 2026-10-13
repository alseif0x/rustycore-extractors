// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original public merge APIs on paired synthetic inputs.
#define main unused_heightfield_driver
#include "native_recast_heightfield.cpp"
#undef main
#include "RecastAlloc.h"
#include <cstdio>
#include <memory>
class MergeContext: public rcContext {
public: MergeContext():rcContext(true){} bool failed=false;
protected: void doLog(rcLogCategory category,char const* message,int length) override {
    if(category==RC_LOG_WARNING||category==RC_LOG_ERROR){failed=true;std::fprintf(stderr,"%.*s\n",length,message);}
}};
template<class T> T* allocate(size_t n) {
    auto p=static_cast<T*>(rcAlloc(n*sizeof(T),RC_ALLOC_PERM));
    if(n&&!p)throw std::bad_alloc();return p;
}
int main(int argc,char** argv){try{
    if(argc!=3)return 2;Input in(argv[1]);Output out(argv[2]);if(in.word()!=0x31474d52)return 3;
    auto count=in.word();if(count>4096)return 4;
    std::vector<std::unique_ptr<rcPolyMesh>> owners;
    using DetailPtr=std::unique_ptr<rcPolyMeshDetail,decltype(&rcFreePolyMeshDetail)>;
    std::vector<DetailPtr> details;
    std::vector<rcPolyMesh*> meshes;std::vector<rcPolyMeshDetail*> dmeshes;
    for(unsigned i=0;i<count;++i){
        auto owner=std::make_unique<rcPolyMesh>();auto& m=*owner;
        m.nverts=in.word();m.npolys=in.word();m.maxpolys=in.word();m.nvp=in.word();m.borderSize=in.word();m.maxEdgeError=in.number();
        for(float& v:m.bmin)v=in.number();for(float& v:m.bmax)v=in.number();m.cs=in.number();m.ch=in.number();
        if(m.nverts<0||m.nverts>65534||m.npolys<0||m.npolys>32768||m.nvp<3||m.nvp>6)return 5;
        m.verts=allocate<unsigned short>(size_t(m.nverts)*3);m.polys=allocate<unsigned short>(size_t(m.npolys)*m.nvp*2);m.regs=allocate<unsigned short>(m.npolys);m.areas=allocate<unsigned char>(m.npolys);m.flags=allocate<unsigned short>(m.npolys);
        for(int j=0;j<m.nverts*3;++j){auto v=in.word();if(v>65535)return 6;m.verts[j]=v;}
        for(int j=0;j<m.npolys;++j){for(int k=0;k<m.nvp*2;++k){auto v=in.word();if(v>65535||(k<m.nvp&&v!=65535&&v>=unsigned(m.nverts)))return 7;m.polys[j*m.nvp*2+k]=v;}auto r=in.word(),a=in.word(),f=in.word();if(r>65535||a>63||f>65535)return 8;m.regs[j]=r;m.areas[j]=a;m.flags[j]=f;}
        DetailPtr detail(rcAllocPolyMeshDetail(),&rcFreePolyMeshDetail);if(!detail)return 9;auto& d=*detail;
        d.nmeshes=in.word();d.nverts=in.word();d.ntris=in.word();if(d.nmeshes!=m.npolys||d.nverts<0||d.nverts>m.npolys*127||d.ntris<0||d.ntris>m.npolys*255)return 10;
        d.meshes=allocate<unsigned int>(size_t(d.nmeshes)*4);d.verts=allocate<float>(size_t(d.nverts)*3);d.tris=allocate<unsigned char>(size_t(d.ntris)*4);
        for(int j=0;j<d.nmeshes*4;++j)d.meshes[j]=in.word();for(int j=0;j<d.nverts*3;++j)d.verts[j]=in.number();for(int j=0;j<d.ntris*4;++j){auto v=in.word();if(v>255)return 11;d.tris[j]=v;}
        (void)in.word(); // Carried source recovery count; merge itself emits no such warning.
        meshes.push_back(owner.get());dmeshes.push_back(detail.get());owners.push_back(std::move(owner));details.push_back(std::move(detail));
    }
    if(in.f.peek()!=std::char_traits<char>::eof())return 12;
    MergeContext ctx;rcPolyMesh merged;DetailPtr detail(rcAllocPolyMeshDetail(),&rcFreePolyMeshDetail);if(!detail)return 13;
    if(!rcMergePolyMeshes(&ctx,meshes.data(),count,merged)||!rcMergePolyMeshDetails(&ctx,dmeshes.data(),count,*detail)||ctx.failed)return 14;
    auto& m=merged;auto& d=*detail;
    out.word(m.nverts);out.word(m.npolys);out.word(m.maxpolys);out.word(m.nvp);out.word(m.borderSize);out.number(m.maxEdgeError);
    for(float v:m.bmin)out.number(v);for(float v:m.bmax)out.number(v);out.number(m.cs);out.number(m.ch);
    for(int j=0;j<m.nverts*3;++j)out.word(m.verts[j]);for(int j=0;j<m.npolys;++j){for(int k=0;k<2*m.nvp;++k)out.word(m.polys[j*2*m.nvp+k]);out.word(m.regs[j]);out.word(m.areas[j]);out.word(m.flags[j]);}
    out.word(d.nmeshes);out.word(d.nverts);out.word(d.ntris);for(int j=0;j<d.nmeshes*4;++j)out.word(d.meshes[j]);for(int j=0;j<d.nverts*3;++j)out.number(d.verts[j]);for(int j=0;j<d.ntris*4;++j)out.word(d.tris[j]);out.f.flush();return out.f?0:15;
}catch(std::exception const&){return 16;}}
