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
