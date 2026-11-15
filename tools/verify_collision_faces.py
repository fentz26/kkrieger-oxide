#!/usr/bin/env python3
"""Original face clipping and complete collision-face generation."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle

root = oracle.root
base = root / '.reference/fr_public/werkkzeug3_kkrieger'
oracle.source = (base / 'genmesh.cpp').read_text(encoding='cp1252')
functions = '\n'.join(oracle.function(s) for s in [
    'void GenSimpleFace::Init(', 'void GenSimpleFace::Exit(',
    'void GenSimpleFace::AddVertex(', 'static sInt ClassifyVert(',
    'ClipCode GenSimpleFace::Clip(',
])
oracle.source = (base / 'kkriegergame.cpp').read_text(encoding='cp1252')
functions += '\n' + '\n'.join(oracle.function(s) for s in [
    'static sBool sMakePlane(', 'void KKriegerCell::Init(const sVector *',
    'void KKriegerGame::CreateCollisionFaces(', 'sBool KKriegerGame::SplitCollisionFace(',
    'void KKriegerCellAdd::DeleteFaces(',
])
support = r'''
#include <vector>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <cassert>
#include <initializer_list>
using sInt=int;using sF32=float;using sF64=double;using sBool=bool;
#define sTRUE true
#define sFALSE false
#define EPSILON (4.010e-3f)
#define OVERLAP (EPSILON*2)
#define KCM_SUB 1
#define KKRIEGER_MAXPLANES 12
#define sVERIFY(x) assert(x);
#define SPLITHEAPMAX 1024
void sSetMem(void*p,int x,int n){memset(p,x,n);}double sFSqrt(double x){return sqrt(x);}
template<class T>T sMin(T a,T b){return a<b?a:b;}template<class T>T sMax(T a,T b){return a>b?a:b;}
struct sVector {float x=0,y=0,z=0,w=0;
 void Init3(){x=y=z=0;w=1;}
 void Sub3(const sVector&a,const sVector&b){x=a.x-b.x;y=a.y-b.y;z=a.z-b.z;}
 float Dot3(const sVector&a)const{return x*a.x+y*a.y+z*a.z;}
 void Cross3(const sVector&a,const sVector&b){x=a.y*b.z-a.z*b.y;y=a.z*b.x-a.x*b.z;z=a.x*b.y-a.y*b.x;}
 void Add3(const sVector&a){x+=a.x;y+=a.y;z+=a.z;}void Scale3(float f){x*=f;y*=f;z*=f;}
 void Lin4(const sVector&a,const sVector&b,float t){x=a.x+(b.x-a.x)*t;y=a.y+(b.y-a.y)*t;z=a.z+(b.z-a.z)*t;w=a.w+(b.w-a.w)*t;}
};
template<class T>struct Array {std::vector<T>v;int Count=0;T&operator[](int i){return v[i];}T*Add(){v.push_back(T{});Count=v.size();return &v.back();}};
enum ClipCode {CC_ALL_ON_PLANE,CC_NOT_CROSSED,CC_CROSSED};
struct GenSimpleFace {int VertexCount;sVector*Vertices;void Init(int);void Exit();void AddVertex(const sVector&);ClipCode Clip(const sVector&,GenSimpleFace*)const;};
struct GenSimpleFaceList:GenSimpleFace {GenSimpleFaceList*Next;};
// Preserve the original Planes[12] immediately followed by Center. The source
// split loop reads planes[PlaneCount], including Center when PlaneCount==12.
struct KKriegerCell {sVector BBMin,BBMax,Planes[12],Center;int PlaneCount,Mode,OnlyForShot;sVector Vertices[8];void Init(const sVector*,int);};
struct KKriegerCellAdd:KKriegerCell {Array<KKriegerCellAdd*>Adds;GenSimpleFaceList*Faces=nullptr;void DeleteFaces();};
struct KKriegerGame {void CreateCollisionFaces(KKriegerCellAdd&);bool SplitCollisionFace(const GenSimpleFace&,const sVector*,int,int,KKriegerCellAdd&);};
sVector SplitHeapVector[2][SPLITHEAPMAX];int SplitHeapIndex[2];
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vector(sVector v){for(float f:{v.x,v.y,v.z,v.w})real(f);}
'''
driver = r'''
int main(){word(128);for(int c=0;c<128;c++){
 KKriegerCellAdd cells[3];word(3);
 for(int n=0;n<3;n++){
  sVector v[8];for(int k=0;k<8;k++){
   float x=-((k&1)-0.5f)*2,y=-(((k>>1)&1)-0.5f)*2,z=-(((k>>2)&1)-0.5f)*2;
   v[k]={x+(n?((c+n)%9-4)*0.5f:0),y+(n==2?0.5f:0),z+(n==1?0.25f:0),1};
   if(n&&c%4==0){v[k].x+=y*0.25f;v[k].z+=x*0.125f;}
   if(n&&c%7==0)v[k].z+=((k*3+c)%5-2)*0.125f;
   if(n&&c%11==0)v[k].y+=((k*5+c)%7-3)*0.125f;
   if(n&&c%13==0)v[k].z=0;
   if(n&&c%19==0){
    static float dx[8]={0.125f,-0.25f,0.0625f,-0.125f,0.1875f,0.25f,-0.0625f,0.3125f};
    static float dy[8]={-0.0625f,0.1875f,-0.125f,0.25f,-0.1875f,0.0625f,0.3125f,-0.25f};
    static float dz[8]={0.25f,-0.125f,-0.0625f,0.1875f,0.0625f,-0.3125f,0.125f,-0.1875f};
    v[k].x+=dx[k];v[k].y+=dy[k];v[k].z+=dz[k];
   }
  }
  word(0);for(auto p:v)vector(p);cells[n].Init(v,0);
 }
 int count=c%3;word(count);for(int n=1;n<=count;n++){word(n);*cells[0].Adds.Add()=&cells[n];}
 KKriegerGame game;game.CreateCollisionFaces(cells[0]);int faces=0;for(auto p=cells[0].Faces;p;p=p->Next)faces++;word(faces);
 for(auto p=cells[0].Faces;p;p=p->Next){word(p->VertexCount);for(int i=0;i<p->VertexCount;i++)vector(p->Vertices[i]);}
 cells[0].DeleteFaces();
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'faces.cpp'
    exe = pathlib.Path(tmp) / 'faces'
    cpp.write_text(support + functions + driver)
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/collision-faces.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 128 original collision-face sets')
