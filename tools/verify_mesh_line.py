#!/usr/bin/env python3
"""Original non-intro MPP_THICKLINES case, including projection and GeoBegin batches."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/engine.cpp').read_text(encoding='cp1252')
function=oracle.function('void EngMesh::PaintJob(')
start=function.index('    case MPP_THICKLINES:',function.index('    case MPP_THICKLINES:')+1)
oracle.source=function[start:]
body=oracle.function('    case MPP_THICKLINES:')
support=r'''
#include <cstdio>
#include <cmath>
#include <vector>
#include <algorithm>
using sInt=int;using sF32=float;
#define MPP_THICKLINES 2
#define sMin std::min
#define sMax std::max
double sFInvSqrt(double f){return 1.0/std::sqrt(f);}
struct sMatrix;
struct sVector{void Rotate34(const sMatrix&,const sVector&);float x,y,z,w;void Scale3(const sVector&a,float s){x=a.x*s;y=a.y*s;z=a.z*s;}};
struct sMatrix{sVector i,j,k,l;void Trans3(){std::swap(i.y,j.x);std::swap(i.z,k.x);std::swap(j.z,k.y);}};
void sVector::Rotate34(const sMatrix&m,const sVector&v){x=m.i.x*v.x+m.j.x*v.y+m.k.x*v.z+m.l.x;y=m.i.y*v.x+m.j.y*v.y+m.k.y*v.z+m.l.y;z=m.i.z*v.x+m.j.z*v.y+m.k.z*v.z+m.l.z;}
struct sVertexTSpace3Big{float x,y,z;};
struct sVertexStandard{float x,y,z,nx,ny,nz,u,v;};
struct Job{int*VertexBuffer;int VertexCount;};
struct Pass{float Size,Aspect;};
std::vector<std::vector<sVertexStandard>>batches;sMatrix matrix;
#define sGT_MODELVIEW 0
struct System{void GetTransform(int,sMatrix&m){m=matrix;}void GeoBegin(int,int n,int,float**vp,int){batches.emplace_back(n);*vp=(float*)batches.back().data();}void GeoEnd(int){}void GeoDraw(int){}void GeoFlush(int){}}system_;System*sSystem=&system_;
void word(unsigned v){std::fwrite(&v,4,1,stdout);}void real(float f){std::fwrite(&f,4,1,stdout);}
void run(Job*job,Pass*pass,sVertexTSpace3Big*vert){int handle=0;switch(MPP_THICKLINES){
'''
driver=r'''
}}
int main(){word(64);for(int c=0;c<64;c++){
int n=c==62?8192:c==63?8194:(c%9)*2;float scale=(c-20)*0.03125f;
float*m=(float*)&matrix;for(int i=0;i<16;i++)m[i]=((c*7+i*13)%31-15)*0.125f;
float aspect=(c%7+1)*0.5f;word(n);real(scale);real(aspect);for(int i=0;i<16;i++)real(m[i]);
std::vector<sVertexTSpace3Big>vertices(n);std::vector<int>indices(n);for(int i=0;i<n;i++){vertices[i]={float(i%23-11)*0.25f,float(i%17-8)*0.125f,float(i%13-6)*0.5f};indices[i]=n-i-1;for(float v:{vertices[i].x,vertices[i].y,vertices[i].z})real(v);}
Job job{indices.data(),n};Pass pass{scale,aspect};run(&job,&pass,vertices.data());
word(batches.size());for(auto&batch:batches){word(batch.size());for(auto&v:batch){const float*f=&v.x;for(int i=0;i<8;i++)real(f[i]);}}batches.clear();}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp=pathlib.Path(tmp)/'sprite.cpp';exe=pathlib.Path(tmp)/'sprite'
    cpp.write_text(support+body+driver)
    subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/mesh-line.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 64 original thick-line finalizer calls')
