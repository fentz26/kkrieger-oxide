#!/usr/bin/env python3
"""Native original collision-cell constructor, warped and degenerate cells."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle

root = oracle.root
oracle.source = (root / '.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
functions = '\n'.join(oracle.function(s) for s in ['static sBool sMakePlane(', 'void KKriegerCell::Init(const sVector *'])
support = r'''
#include <cmath>
#include <cstdio>
#include <cstring>
#include <cassert>
#include <initializer_list>
using sInt=int;using sF32=float;using sF64=double;using sBool=bool;
#define sTRUE true
#define sFALSE false
#define KKRIEGER_MAXPLANES 12
#define KCM_SUB 1
#define OVERLAP (4.010e-3f*2)
#define sVERIFY assert
void sSetMem(void*p,int x,int n){memset(p,x,n);}
double sFSqrt(double x){return sqrt(x);}
template<class T>T sMin(T a,T b){return a<b?a:b;}template<class T>T sMax(T a,T b){return a>b?a:b;}
struct sVector {float x,y,z,w;
 void Sub3(const sVector&a,const sVector&b){x=a.x-b.x;y=a.y-b.y;z=a.z-b.z;}
 float Dot3(const sVector&a)const{return x*a.x+y*a.y+z*a.z;}
 void Cross3(const sVector&a,const sVector&b){x=a.y*b.z-a.z*b.y;y=a.z*b.x-a.x*b.z;z=a.x*b.y-a.y*b.x;}
 void Add3(const sVector&a){x+=a.x;y+=a.y;z+=a.z;}void Scale3(float f){x*=f;y*=f;z*=f;}
};
struct KKriegerCell {sVector Vertices[8],Planes[12];int Mode,OnlyForShot,PlaneCount;sVector Center,BBMin,BBMax;void Init(const sVector*,int);};
void word(unsigned x){fwrite(&x,4,1,stdout);}void real(float x){fwrite(&x,4,1,stdout);}void vector(sVector v){for(float x:{v.x,v.y,v.z,v.w})real(x);}
'''
driver = r'''
int main(){word(256);for(int c=0;c<256;c++){
 sVector v[8];for(int k=0;k<8;k++){
  float x=-((k&1)-0.5f)*(c%7-2),y=-(((k>>1)&1)-0.5f)*((c/7)%7-2),z=-(((k>>2)&1)-0.5f)*((c/49)%7-2);
  v[k]={x+y*0.25f+0.125f,y-z*0.125f-0.25f,z+x*0.5f+0.5f,1};
 }
 if(c%3==0)v[7].z+=0.125f;if(c%5==0)v[6].y-=0.25f;
 if(c%11==0)v[0]=v[2];if(c%17==0)for(int k=0;k<8;k++)v[k].z=0;
 word(c%8);for(auto p:v)vector(p);
 KKriegerCell cell;cell.Init(v,c%8);word(cell.Mode);word(cell.OnlyForShot);word(cell.PlaneCount);
 for(int i=0;i<cell.PlaneCount;i++)vector(cell.Planes[i]);vector(cell.Center);vector(cell.BBMin);vector(cell.BBMax);
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'collision.cpp'
    exe = pathlib.Path(tmp) / 'collision'
    cpp.write_text(support + functions + driver)
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/collision.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 256 original cells')

dynamic_functions = '\n'.join(oracle.function(s) for s in [
    'void KKriegerCellDynamic::PrepareDynamic(',
    'void KKriegerCellDynamic::ApplyMatrix(',
])
oracle.source = (root / '.reference/fr_public/werkkzeug3_kkrieger/_types.cpp').read_text(encoding='cp1252')
dynamic_functions += '\n' + oracle.function('void sMatrix::Lin4(')
dynamic_support = support.replace('struct sVector {', 'struct sMatrix;\nstruct sVector {')
dynamic_support = dynamic_support.replace(' void Sub3(', '''
 void Lin3(const sVector&a,const sVector&b,float t){x=a.x+(b.x-a.x)*t;y=a.y+(b.y-a.y)*t;z=a.z+(b.z-a.z)*t;}
 void Unit3(){Scale3(float(1.0/sqrt(double(Dot3(*this)))));}
 void Scale4(float f){x*=f;y*=f;z*=f;w*=f;}
 float Abs3(){return sqrt(double(Dot3(*this)));}
 void Rotate3(const sMatrix&,const sVector&);void RotatePl(const sMatrix&,const sVector&);
 void Sub3(''')
dynamic_support += r'''
#define KCM_UNCONFIRMED 0x20
struct sMatrix {sVector i,j,k,l;void Init(){i={1,0,0,0};j={0,1,0,0};k={0,0,1,0};l={0,0,0,1};}void Lin4(sMatrix&,sMatrix&,float);};
void sVector::Rotate3(const sMatrix&m,const sVector&v){x=m.i.x*v.x;y=m.i.y*v.x;z=m.i.z*v.x;x+=m.j.x*v.y;y+=m.j.y*v.y;z+=m.j.z*v.y;x+=m.k.x*v.z;y+=m.k.y*v.z;z+=m.k.z*v.z;}
void sVector::RotatePl(const sMatrix&m,const sVector&v){Rotate3(m,v);w=v.w-Dot3(m.l);}
struct KKriegerCellDynamic:KKriegerCell {sVector OrigPlanes[6],OrigVertices[8];sMatrix ConfirmedMatrix,CurrentMatrix,Matrix0,Matrix1;int Blocked;void*Monster;void PrepareDynamic(sMatrix&);void ApplyMatrix(sMatrix&);};
void matrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vector(v);}
void dump(KKriegerCellDynamic&c){word(c.Mode);word(c.Blocked);word(c.Monster!=nullptr);for(int i=0;i<c.PlaneCount;i++)vector(c.Planes[i]);for(auto v:c.Vertices)vector(v);vector(c.Center);vector(c.BBMin);vector(c.BBMax);for(auto m:{c.ConfirmedMatrix,c.CurrentMatrix,c.Matrix0,c.Matrix1})matrix(m);}
'''
dynamic_driver = r'''
int main(){word(128);for(int c=0;c<128;c++){
 sVector v[8];for(int k=0;k<8;k++)v[k]={-((k&1)-0.5f)*(1+c%5),-(((k>>1)&1)-0.5f)*(1+c%3),-(((k>>2)&1)-0.5f)*(1+c%7),1};
 for(auto p:v)vector(p);
 sMatrix a,b;a.Init();b.Init();a.i.y=(c%3)*0.25f;a.j.z=-0.125f;a.l={float(c%7),-0.25f,0.5f,1};
 b.i={0.75f,0.25f,-0.125f,0};b.j={-0.5f,1.25f,0.25f,0};b.k={0.25f,0,1.5f,0};b.l={-0.5f,float(c%11),0.75f,1};
 matrix(a);matrix(b);float fade=(c%9-2)*0.25f;real(fade);
 KKriegerCellDynamic cell;cell.Init(v,1);cell.PrepareDynamic(a);dump(cell);
 cell.Blocked=1;cell.ApplyMatrix(b);dump(cell);
 cell.Matrix1=b;cell.CurrentMatrix.Lin4(cell.Matrix0,cell.Matrix1,fade);cell.ApplyMatrix(cell.CurrentMatrix);cell.ConfirmedMatrix=cell.CurrentMatrix;cell.Blocked=0;cell.Mode&=~KCM_UNCONFIRMED;dump(cell);
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'dynamic.cpp'
    exe = pathlib.Path(tmp) / 'dynamic'
    cpp.write_text(dynamic_support + functions + dynamic_functions + dynamic_driver)
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/collision-dynamic.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 128 original dynamic cells')

oracle.source = (root / '.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
query_functions = '\n'.join(oracle.function(s) for s in [
    'sBool KKriegerCell::RejectPlane(', 'sInt KKriegerCell::OutsideMask(',
    'sBool KKriegerCell::IntersectOut(', 'sBool KKriegerCell::IntersectIn(',
    'sF32 KKriegerCell::approxDistance(',
])
query_support = support.replace('void Init(const sVector*,int);', '''
void Init(const sVector*,int);bool RejectPlane(const sVector&,const sVector&);
int OutsideMask(const sVector&,float);bool IntersectOut(const sVector&,const sVector&,sVector&,float&,float);
bool IntersectIn(const sVector&,const sVector&,sVector&,float&,float);float approxDistance(const sVector&);
''')
query_support = '#define GAMEPERF(x)\n#define LOGGING 0\n' + query_support
query_driver = r'''
int main(){word(256);word(16);for(int c=0;c<256;c++){
 sVector v[8];for(int k=0;k<8;k++)v[k]={-((k&1)-0.5f)*(c%7-2),-(((k>>1)&1)-0.5f)*((c/7)%7-2),-(((k>>2)&1)-0.5f)*((c/49)%7-2),1};
 if(c%3==0)v[7].z+=0.125f;if(c%5==0)v[6].y-=0.25f;
 if(c%11==0)v[0]=v[2];if(c%17==0)for(int k=0;k<8;k++)v[k].z=0;
 word(c%8);for(auto p:v)vector(p);KKriegerCell cell;cell.Init(v,c%8);
 for(int k=0;k<16;k++){
  sVector p={(k%5-2)*0.5f,((c+k)%7-3)*0.25f,((k+c/7)%9-4)*0.25f,1};
  sVector d={((c+k)%5-2)*0.25f,(k%3-1)*0.5f,((c/3+k)%7-3)*0.25f,0};
  if(k==0)d={0,0,0,0};if(k==1)d={1e-16f,-1e-16f,0,0};if(k==2)d={-0.0f,0,-0.0f,0};
  sVector b={p.x+d.x,p.y+d.y,p.z+d.z,1};float radius=(k%5-1)*0.125f;
  vector(p);vector(d);vector(b);real(radius);word(cell.OutsideMask(p,radius));word(cell.RejectPlane(p,b));real(cell.approxDistance(p));
  sVector plane{};float distance=0;bool hit=cell.IntersectOut(p,d,plane,distance,radius);word(hit);if(hit){vector(plane);real(distance);}
  hit=cell.IntersectIn(p,d,plane,distance,radius);word(hit);if(hit){vector(plane);real(distance);}
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'queries.cpp'
    exe = pathlib.Path(tmp) / 'queries'
    cpp.write_text(query_support + functions + query_functions + query_driver)
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/collision-queries.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 4096 original cell queries')
