#!/usr/bin/env python3
"""Original paint-job AABB, sphere and frustum calculations."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_types.cpp').read_text(encoding='cp1252')
functions='\n'.join(oracle.function(name) for name in ['void sAABox::Rotate34(','sBool sAABox::IntersectsSphere(','void sCullPlane::LinearComb(','void sCullPlane::CalcNVert()','sF32 sCullPlane::Distance(','sBool sCullBBox(','void sFrustum::FromViewProject(','void sMatrix::Trans4()'])
support=r'''
#include <cstdio>
#include <algorithm>
#include <initializer_list>
using sInt=int;using sF32=float;using sBool=bool;
#define sTRUE true
#define sFALSE false
#define sSwap std::swap
struct sMatrix;
struct sVector{float x,y,z,w;void Scale3(const sVector&a,float s){x=a.x*s;y=a.y*s;z=a.z*s;}void Scale4(const sVector&a,float s){x=a.x*s;y=a.y*s;z=a.z*s;w=a.w*s;}void AddScale4(const sVector&a,float s){x+=a.x*s;y+=a.y*s;z+=a.z*s;w+=a.w*s;}void Add4(const sVector&a){x+=a.x;y+=a.y;z+=a.z;w+=a.w;}float Dot3(const sVector&a)const{return x*a.x+y*a.y+z*a.z;}void Rotate34(const sMatrix&,const sVector&);};
struct sMatrix{sVector i,j,k,l;void Trans4();};
void sVector::Rotate34(const sMatrix&m,const sVector&v){Scale4(m.i,v.x);AddScale4(m.j,v.y);AddScale4(m.k,v.z);Add4(m.l);}
struct sAABox{sVector Min,Max;void Rotate34(const sAABox&,const sMatrix&);bool IntersectsSphere(const sVector&,float);};
struct sCullPlane{sVector Plane;int NVert;void LinearComb(const sVector&,float,const sVector&,float);void CalcNVert();float Distance(const sVector&)const;};
struct sFRect{float x0,y0,x1,y1;};struct sFrustum{sCullPlane Planes[6];int NPlanes;void FromViewProject(const sMatrix&,const sFRect&);};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vec(sVector v){for(float f:{v.x,v.y,v.z,v.w})real(f);}void xyz(sVector v){for(float f:{v.x,v.y,v.z})real(f);}
unsigned seed=1234567;float rnd(){seed=seed*1664525+1013904223;return int(seed>>16)/8192.f-4.f;}sVector vector(){return {rnd(),rnd(),rnd(),rnd()};}
'''
driver=r'''
int main(){word(512);for(int n=0;n<512;n++){
sAABox bounds{vector(),vector()};bounds.Max.x=bounds.Min.x+(n%5)*.25f;bounds.Max.y=bounds.Min.y+(n%7)*.5f;bounds.Max.z=bounds.Min.z+(n%11)*.125f;
sMatrix matrix{vector(),vector(),vector(),vector()},projection{vector(),vector(),vector(),vector()};if(n%8==0)projection={{1,0,0,0},{0,1,0,0},{0,0,1,1},{0,0,-.125f,0}};
sFRect viewport{-.75f,-.5f,1.f,.875f};sVector center=vector();float radius=rnd();
xyz(bounds.Min);xyz(bounds.Max);for(auto v:{matrix.i,matrix.j,matrix.k,matrix.l,projection.i,projection.j,projection.k,projection.l})vec(v);for(float f:{viewport.x0,viewport.y0,viewport.x1,viewport.y1})real(f);xyz(center);real(radius);
sAABox world;world.Rotate34(bounds,matrix);sFrustum frustum;frustum.FromViewProject(projection,viewport);
xyz(world.Min);xyz(world.Max);for(auto p:frustum.Planes){vec(p.Plane);word(p.NVert);}word(frustum.NPlanes);word(world.IntersectsSphere(center,radius));word(sCullBBox(world,frustum.Planes,5));word(sCullBBox(world,frustum.Planes,frustum.NPlanes));}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp=pathlib.Path(tmp)/'culling.cpp';exe=pathlib.Path(tmp)/'culling'
    cpp.write_text(support+functions+driver)
    subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/paint-culling.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 512 original box/frustum/light-sphere cases')
