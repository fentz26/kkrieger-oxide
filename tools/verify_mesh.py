#!/usr/bin/env python3
"""Execute pinned, unchanged original mesh initialization and Ring in a native oracle."""
import pathlib
import subprocess
import tempfile

root = pathlib.Path(__file__).resolve().parents[1]
source = (root / '.reference/fr_public/werkkzeug3_kkrieger/genmesh.cpp').read_text(encoding='cp1252')

def function(signature):
    start = source.index(signature)
    body = source.index('{', start)
    depth = 1
    end = body + 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return source[start:end]

support = r'''
#include <vector>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cassert>
#include <cstring>
#define sGMF_NORMALS 6
void sCopyMem4(uint32_t*d,uint32_t*s,int n){memcpy(d,s,n*4);}
using sInt=int;using sU32=uint32_t;using sU8=uint8_t;using sF32=float;using sBool=bool;
#define MSM_ADD 0
#define MSM_SUB 1
#define MSM_SET 2
#define MSM_SETNOT 3
#define sVERIFY assert
#define sGMI_UV0 4
const float sPI2F=6.2831853071795864769f;
// Match _types.hpp: these helpers take and return sF64.
double sFSin(double f){return sin(f);}double sFCos(double f){return cos(f);}
struct sVector{float x=0,y=0,z=0,w=0;};
struct GenMeshElem{sU8 Mask,Id,Select,Used;void InitElem();void SelElem(sU32,sBool,sInt);};
struct GenMeshVert:GenMeshElem{int Next,First,Temp,Temp2,ReIndex;sU8 Weight[4],Matrix[4];void Init();void Sel(sU32 m,bool s,int mode){SelElem(m>>16,s,mode);}};
struct GenMeshEdge:GenMeshElem{int Next[2],Prev[2],Face[2],Vert[2],Temp[2],Crease;void Init();void Sel(sU32 m,bool s,int mode){SelElem(m,s,mode);}};
struct GenMeshFace:GenMeshElem{int Material,Edge,Temp,Temp2,Temp3;void Init();void Sel(sU32 m,bool s,int mode){SelElem(m>>8,s,mode);}};
template<class T>struct Storage{std::vector<T> data;T*Array=nullptr;int Count=0;void AtLeast(int n){if(n>(int)data.size())data.resize(n);Array=data.data();}T*Add(){AtLeast(++Count);return &data[Count-1];}T&operator[](int i){return data[i];}};
struct GenMesh{Storage<GenMeshVert>Vert;Storage<GenMeshEdge>Edge;Storage<GenMeshFace>Face;std::vector<sVector>storage;sVector*VertBuf;
 GenMesh():storage(131072),VertBuf(storage.data()){}int VertSize(){return 5;}int VertMap(int n){return n;}
 int AddVert(){int i=Vert.Count++;Vert.AtLeast(Vert.Count);return i;}
 int AddNewVert();int AddCopiedVert(int);void CopyVert(int,int);
 GenMeshVert*GetVert(int h){return &Vert[Edge[h/2].Vert[h&1]];}
 GenMeshFace*GetFace(int h){return &Face[Edge[h/2].Face[h&1]];}
 GenMeshFace*GetFaceI(int h){return &Face[Edge[h/2].Face[~h&1]];}
 GenMeshEdge*GetEdge(int h){return &Edge[h/2];}
 int GetVertId(sU32);int GetFaceId(sU32);int NextFaceEdge(sU32);int PrevFaceEdge(sU32);int NextVertEdge(sU32);int PrevVertEdge(sU32);
 void SplitFace(sU32,sU32,sU32=0,int=0);void SplitBridge(sU32,sU32,int&,int&,sU32,int,int* =nullptr,int* =nullptr);void FixVertCycle(int);bool IsBorderEdge(sU32,int);void Extrude(sU32,int,int);
 void All2Mask(sU32,int=0);void Ring(int,sU32,float,float,int);
};
void word(uint32_t w){fwrite(&w,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
void element(GenMeshElem&e){word(e.Mask);word(e.Id);word(e.Select);word(e.Used);}
int main(){word(256);for(int c=0;c<256;c++){
 int segments=3+c%14,arc=c%segments;uint32_t mask=0xa53619u^uint32_t(c*0x9171);float radius=0.125f*(1+c%9),phase=0.0625f*(c-23);
 word(segments);word(mask);real(radius);real(phase);word(arc);GenMesh m;m.Ring(segments,mask,radius,phase,arc);
 int creased=c>=192;word(creased);if(creased)for(int j=0;j<m.Edge.Count;j++)m.Edge[j].Crease=(j%2)?6:16;
 int stages=c>=192?2:c/64;word(stages);for(int step=0;step<stages;step++){
 for(int j=0;j<m.Face.Count;j++)m.Face[j].Select=(j==0);m.Extrude(mask, c%4, c%2);
 }
 word(m.Vert.Count);word(m.Edge.Count);word(m.Face.Count);
 for(int i=0;i<m.Vert.Count;i++){auto&v=m.Vert[i];element(v);for(int n:{v.Next,v.First,v.Temp,v.Temp2,v.ReIndex})word(n);for(auto n:v.Weight)word(n);for(auto n:v.Matrix)word(n);for(int j=0;j<5;j++){auto&a=m.VertBuf[i*5+j];real(a.x);real(a.y);real(a.z);real(a.w);}}
 for(int i=0;i<m.Edge.Count;i++){auto&e=m.Edge[i];element(e);for(auto*a:{e.Next,e.Prev,e.Face,e.Vert,e.Temp})for(int j=0;j<2;j++)word(a[j]);word(e.Crease);}
 for(int i=0;i<m.Face.Count;i++){auto&f=m.Face[i];element(f);for(int n:{f.Material,f.Edge,f.Temp,f.Temp2,f.Temp3})word(n);}
}}
'''
functions = '\n'.join(function(s) for s in ['void GenMeshElem::InitElem(', 'void GenMeshElem::SelElem(', 'void GenMeshVert::Init(', 'void GenMeshEdge::Init(', 'void GenMeshFace::Init(', 'sInt GenMesh::AddNewVert(', 'void GenMesh::All2Mask(', 'void GenMesh::Ring(', 'sInt GenMesh::AddCopiedVert(', 'void GenMesh::CopyVert(', 'sInt GenMesh::GetVertId(', 'sInt GenMesh::GetFaceId(', 'sInt GenMesh::NextFaceEdge(', 'sInt GenMesh::PrevFaceEdge(', 'sInt GenMesh::NextVertEdge(', 'sInt GenMesh::PrevVertEdge(', 'void GenMesh::SplitFace(', 'void GenMesh::SplitBridge(', 'void GenMesh::FixVertCycle(', 'sBool GenMesh::IsBorderEdge(', 'void GenMesh::Extrude('])
def main():
    with tempfile.TemporaryDirectory() as tmp:
        cpp, exe = pathlib.Path(tmp)/'mesh.cpp', pathlib.Path(tmp)/'mesh'
        cpp.write_text(support + functions)
        subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
        data = subprocess.check_output([str(exe)])
    out = root/'tests/fixtures/mesh-ring.bin'
    out.write_bytes(data)
    print(f'{out}: 256 original rings, creases and repeated extrusions, complete topology and attributes')

if __name__ == '__main__':
    main()
