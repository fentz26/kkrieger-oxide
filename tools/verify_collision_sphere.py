#!/usr/bin/env python3
"""Original sphere-edge/face checks, including false-result output mutations."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
import verify_collision_world as world

root = oracle.root
oracle.source = (root / '.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
functions = world.functions + '\n' + '\n'.join(oracle.function(s) for s in [
    'void KKriegerGame::CollideSoftSphereFace(', 'sBool KKriegerGame::CheckSphereVsFace(',
    'sBool KKriegerGame::CheckSphereVsEdge(',
])
support = world.support.replace(' void Sub3(', '''
 float Abs3()const{return float(sqrt(double(Dot3(*this))));}
 void Unit3(){Scale3(float(1.0/sqrt(double(Dot3(*this)))));}
 void Scale3(const sVector&a,float f){x=a.x*f;y=a.y*f;z=a.z*f;}
 void AddScale3(const sVector&a,float f){x+=a.x*f;y+=a.y*f;z+=a.z*f;}
 float Distance(const sVector&a)const{sVector v;v.Sub3(*this,a);return v.Abs3();}
 void Sub3(''')
support = support.replace('struct KKriegerGame {', '''struct KKriegerGame {
 void CollideSoftSphereFace(sVector&,const sVector*,int);
 bool CheckSphereVsFace(const sVector&,const sVector*,int,sVector&);
 bool CheckSphereVsEdge(const sVector&,const sVector&,const sVector&,sVector&);
''')
driver = r'''
void point(sVector v){for(float x:{v.x,v.y,v.z})real(x);}
int main(){word(512);word(8);KKriegerGame game;for(int c=0;c<512;c++){
 int count=3+c%5;sVector v[8];static float xs[8]={-1,-0.75f,0,0.75f,1,0.75f,0,-0.75f},ys[8]={0,-0.75f,-1,-0.75f,0,0.75f,1,0.75f};
 for(int i=0;i<count;i++){
  float x=xs[i]*(1+c%3)*0.5f,y=ys[i]*(1+c%5)*0.25f,z=0;
  if(c%7==0)z=(i%3)*0.0625f;
  if(c%3==0)v[i]={x,y,z,1};else if(c%3==1)v[i]={y,z,x,1};else v[i]={z,x,y,1};
 }
 if(c%11==0)v[1]=v[0];if(c%13==0)for(int i=0;i<count;i++)v[i]={float(i)*0.25f,0,0,1};
 word(count);for(int i=0;i<count;i++)vector(v[i]);
 for(int k=0;k<8;k++){
  sVector sphere={(k%4-1)*0.5f,((k+c)%5-2)*0.25f,((k+c/5)%5-2)*0.125f,(k%4)*0.25f};
  if(k==0)sphere={0,0,0,0.5f};if(k==1)sphere=v[0],sphere.w=0.5f;
  vector(sphere);sVector nearest{11,12,13,14};bool hit=game.CheckSphereVsFace(sphere,v,count,nearest);word(hit);point(nearest);
  sVector response=sphere;game.CollideSoftSphereFace(response,v,count);vector(response);
  nearest={11,12,13,14};int edge=k%count;word(edge);hit=game.CheckSphereVsEdge(sphere,v[edge],v[(edge+1)%count],nearest);word(hit);point(nearest);
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'sphere.cpp'
    exe = pathlib.Path(tmp) / 'sphere'
    cpp.write_text(support + functions + driver)
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/collision-sphere.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 4096 original sphere/face/edge cases')
