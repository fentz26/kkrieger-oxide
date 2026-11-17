#!/usr/bin/env python3
"""Original MPP_SPRITES case, including unnormalized axes and GeoBegin batches."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/engine.cpp').read_text(encoding='cp1252')
function=oracle.function('void EngMesh::PaintJob(')
start=function.index('    case MPP_SPRITES:',function.index('    case MPP_SPRITES:')+1)
oracle.source=function[start:]
body=oracle.function('    case MPP_SPRITES:')
support=r'''
#include <cstdio>
#include <vector>
#include <algorithm>
using sInt=int;using sF32=float;
#define MPP_SPRITES 2
#define sMin std::min
struct sVector{float x,y,z,w;void Scale3(const sVector&a,float s){x=a.x*s;y=a.y*s;z=a.z*s;}};
struct sMatrix{sVector i,j,k,l;void Trans3(){std::swap(i.y,j.x);std::swap(i.z,k.x);std::swap(j.z,k.y);}};
struct sVertexTSpace3Big{float x,y,z;};
struct sVertexStandard{float x,y,z,nx,ny,nz,u,v;};
struct Job{int*VertexBuffer;int VertexCount;};
struct Pass{float Size,Aspect;};
std::vector<std::vector<sVertexStandard>>batches;sMatrix matrix;
#define sGT_MODELVIEW 0
struct System{void GetTransform(int,sMatrix&m){m=matrix;}void GeoBegin(int,int n,int,float**vp,int){batches.emplace_back(n);*vp=(float*)batches.back().data();}void GeoEnd(int){}void GeoDraw(int){}void GeoFlush(int){}}system_;System*sSystem=&system_;
void word(unsigned v){std::fwrite(&v,4,1,stdout);}void real(float f){std::fwrite(&f,4,1,stdout);}
void run(Job*job,Pass*pass,sVertexTSpace3Big*vert){int handle=0;switch(MPP_SPRITES){
'''
driver=r'''
}}
int main(){word(64);for(int c=0;c<64;c++){
int n=c==62?4096:c==63?4097:c%9;float scale=(c-20)*0.03125f;
float*m=(float*)&matrix;for(int i=0;i<16;i++)m[i]=((c*7+i*13)%31-15)*0.125f;
word(n);real(scale);for(int i=0;i<16;i++)real(m[i]);
std::vector<sVertexTSpace3Big>vertices(n);std::vector<int>indices(n);for(int i=0;i<n;i++){vertices[i]={float(i%23-11)*0.25f,float(i%17-8)*0.125f,float(i%13-6)*0.5f};indices[i]=n-i-1;for(float v:{vertices[i].x,vertices[i].y,vertices[i].z})real(v);}
Job job{indices.data(),n};Pass pass{scale,2.f};run(&job,&pass,vertices.data());
word(batches.size());for(auto&batch:batches){word(batch.size());for(auto&v:batch){const float*f=&v.x;for(int i=0;i<8;i++)real(f[i]);}}batches.clear();}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp=pathlib.Path(tmp)/'sprite.cpp';exe=pathlib.Path(tmp)/'sprite'
    cpp.write_text(support+body+driver)
    subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/mesh-sprite.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 64 original sprite finalizer calls')
