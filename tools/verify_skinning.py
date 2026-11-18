#!/usr/bin/env python3
"""Run the pinned original EngMesh::EvalBones against deterministic weighted vertices."""
import pathlib, subprocess, tempfile
import verify_mesh as oracle
root = oracle.root
oracle.source = (root / '.reference/fr_public/werkkzeug3_kkrieger/engine.cpp').read_text(encoding='cp1252')
body = oracle.function('void *EngMesh::EvalBones(')
support = r'''
#include <cstdio>
#include <cmath>
#include <cstdlib>
using sF32=float;
#define sZONE(x)
double sFSqrt(double x){return sqrt(x);}
struct V {float x,y,z,w;};
struct sMatrix {V i,j,k,l;};
struct sVertexTSpace3Big {float x,y,z,nx,ny,nz,sx,sy,sz;unsigned c;float u,v;};
struct sGrowableMemStack {template<class T>T*Alloc(int n){return new T[n];}};
struct EngMesh {
 struct BoneInfo {unsigned char BoneCount;float Weight[4];unsigned short Matrix[4];};
 BoneInfo*BoneInfos; sVertexTSpace3Big*Vert;int VertCount;
 void*EvalBones(sMatrix*,sGrowableMemStack&);
};
unsigned seed=1234567;
unsigned randomword(){seed=seed*196314165u+907633515u;return seed;}
float value(){return (int(randomword()%2049)-1024)*0.00390625f;}
void word(unsigned v){fwrite(&v,4,1,stdout);}
void real(float v){fwrite(&v,4,1,stdout);}
void vertex(const sVertexTSpace3Big&v){const float*p=&v.x;for(int i=0;i<9;i++)real(p[i]);word(v.c);real(v.u);real(v.v);}
'''
driver = r'''
int main(){word(256);for(int c=0;c<256;c++){
 sMatrix matrices[4];for(auto&m:matrices){float*p=&m.i.x;for(int i=0;i<16;i++)p[i]=value();}
 sVertexTSpace3Big vertices[8];EngMesh::BoneInfo info[8];
 for(int v=0;v<8;v++){
  float*p=&vertices[v].x;for(int i=0;i<9;i++)p[i]=value();
  vertices[v].c=randomword();vertices[v].u=value();vertices[v].v=value();
  if(c%16==0){vertices[v].nx=vertices[v].ny=vertices[v].nz=0;}
  if(c%17==0){vertices[v].sx=vertices[v].sy=vertices[v].sz=0;}
  info[v].BoneCount=(c+v)%5;
  for(int i=0;i<4;i++){info[v].Weight[i]=value();info[v].Matrix[i]=randomword()%4;}
 }
 for(auto&m:matrices){float*p=&m.i.x;for(int i=0;i<16;i++)real(p[i]);}
 for(int v=0;v<8;v++){vertex(vertices[v]);word(info[v].BoneCount);for(int i=0;i<4;i++)real(info[v].Weight[i]);for(int i=0;i<4;i++)word(info[v].Matrix[i]);}
 EngMesh mesh{info,vertices,8};sGrowableMemStack stack;
 auto*out=(sVertexTSpace3Big*)mesh.EvalBones(matrices,stack);
 for(int v=0;v<8;v++)vertex(out[v]);delete[]out;
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp=pathlib.Path(tmp)/'skinning.cpp'; exe=pathlib.Path(tmp)/'skinning'
    cpp.write_text(support+body+driver)
    subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
output=root/'tests/fixtures/skinning.bin'
output.write_bytes(data)
print(f'{output}: {len(data)} bytes, 2048 original deformed vertices')
