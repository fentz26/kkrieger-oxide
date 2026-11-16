#!/usr/bin/env python3
"""Original light ZFailVolume, enlargement and normalization functions."""
import pathlib
import subprocess
import tempfile
import verify_paint_culling as culling
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_types.cpp').read_text(encoding='cp1252')
functions='\n'.join(oracle.function(name) for name in ['void sVector::Plane(','void sCullPlane::From3Points(','void sCullPlane::Normalize()','void sFrustum::ZFailVolume(','void sFrustum::EnlargeToInclude(','void sFrustum::Normalize()'])
support=culling.support.replace('#include <cstdio>','#include <cstdio>\n#include <cmath>')
support=support.replace('float x,y,z,w;',r'''float x,y,z,w;
void Sub3(const sVector&a,const sVector&b){x=a.x-b.x;y=a.y-b.y;z=a.z-b.z;}
void AddScale3(const sVector&a,float s){x+=a.x*s;y+=a.y*s;z+=a.z*s;}
void Scale4(float s){x*=s;y*=s;z*=s;w*=s;}
void Cross3(const sVector&a,const sVector&b){x=a.y*b.z-a.z*b.y;y=a.z*b.x-a.x*b.z;z=a.x*b.y-a.y*b.x;}
void Plane(const sVector&,const sVector&,const sVector&);
''')
support=support.replace('float Distance(const sVector&)const;','float Distance(const sVector&)const;void From3Points(const sVector&,const sVector&,const sVector&);void Normalize();')
support=support.replace('void FromViewProject(const sMatrix&,const sFRect&);','void FromViewProject(const sMatrix&,const sFRect&);void ZFailVolume(const sVector&,const sMatrix&,float,float[2],const sFRect&);void EnlargeToInclude(const sVector&);void Normalize();')
support+='\ndouble sFInvSqrt(double x){return 1./sqrt(x);}double sFAbs(double x){return fabs(x);}\n'
driver=r'''
void emit(const sFrustum&f){word(f.NPlanes);for(int i=0;i<f.NPlanes;i++){vec(f.Planes[i].Plane);word(f.Planes[i].NVert);}sAABox box{{-1,-1,-1,1},{1,1,1,1}};word(sCullBBox(box,f.Planes,f.NPlanes));}
int main(){word(512);for(int c=0;c<512;c++){
sMatrix view{{1,0,0,0},{0,1,0,0},{0,0,1,0},{(c%7)*.125f,(c%11)*.25f,(c%13)*.0625f,1}};
float cs[]={1,0,-1,0},sn[]={0,1,0,-1};int turn=(c/8)%4;view.i={cs[turn],0,-sn[turn],0};view.k={sn[turn],0,cs[turn],0};
float near=.125f+(c%7)*.125f,zoom[]={.75f+(c%5)*.25f,.5f+(c%11)*.125f},offsets[]={0,-near,near,1e-5f,-1e-5f,1e-4f,-1e-4f,2.5f};sFRect viewport{-.75f,-.5f,.875f,1};sVector light=view.l;light.AddScale3(view.i,(c%3)*.25f);light.AddScale3(view.k,near+offsets[c%8]);sVector point{(c%9-4)*.75f,(c%7-3)*.5f,(c%11-5)*.625f,1};
xyz(light);for(auto v:{view.i,view.j,view.k,view.l})vec(v);for(float f:{near,zoom[0],zoom[1],viewport.x0,viewport.y0,viewport.x1,viewport.y1})real(f);xyz(point);
sFrustum f{};f.ZFailVolume(light,view,near,zoom,viewport);emit(f);f.EnlargeToInclude(point);emit(f);f.Normalize();emit(f);}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp=pathlib.Path(tmp)/'light.cpp';exe=pathlib.Path(tmp)/'light'
    cpp.write_text(support+culling.functions+functions+driver)
    subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/light-frustum.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 512 original light frustum sequences')
