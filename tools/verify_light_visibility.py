#!/usr/bin/env python3
"""Original CalcSphereBounds and both AddLightJob fade configurations."""
import pathlib
import subprocess
import tempfile
import verify_paint_culling as culling
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_types.cpp').read_text(encoding='cp1252')
normalize='\n'.join(oracle.function(n) for n in ['void sCullPlane::Normalize()','void sFrustum::Normalize()'])
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/engine.cpp').read_text(encoding='cp1252')
sphere=oracle.function('static void CalcSphereBounds(')
add=oracle.function('void Engine_::AddLightJob(')
support=culling.support.replace('#include <cstdio>','#include <cstdio>\n#include <cmath>')
support=support.replace('float x,y,z,w;',r'''float x,y,z,w;
float operator[](int i)const{return (&x)[i];}
float Distance(const sVector&a)const{float dx=x-a.x,dy=y-a.y,dz=z-a.z;return float(sqrt(double(dx*dx+dy*dy+dz*dz)));}
void Scale4(float s){x*=s;y*=s;z*=s;w*=s;}
''')
support=support.replace('float Distance(const sVector&)const;','float Distance(const sVector&)const;void Normalize();')
support=support.replace('void FromViewProject(const sMatrix&,const sFRect&);','void FromViewProject(const sMatrix&,const sFRect&);void Normalize();')
support=support.replace('float x0,y0,x1,y1;', 'float x0,y0,x1,y1;void Init(float a,float b,float c,float d){x0=a;y0=b;x1=c;y1=d;}')
support+=r'''
double sFInvSqrt(double x){return 1./sqrt(x);}double sFSqrt(double x){return sqrt(x);}
template<class T>T sMax(T a,T b){return a>b?a:b;}template<class T>T sMin(T a,T b){return a<b?a:b;}template<class T>T sRange(T a,T high,T low){return a>=high?high:a<=low?low:a;}
#define sINTRO 1
struct EngLight{sVector Position;float Range,Amplify;};
struct Engine_{sFrustum Frustum;struct{sMatrix CameraSpace;}Env;bool Selected=false;float Importance,Fade;
void InsertLightJob(const EngLight&,float importance,float fade){Selected=true;Importance=importance;Fade=fade;}
void AddLightJob(const EngLight&);void AddLightJobNonK(const EngLight&);};
'''
driver=r'''
void emit(Engine_&e){word(e.Selected);if(e.Selected){real(e.Importance);real(e.Fade);}e.Selected=false;}
int main(){word(512);for(int c=0;c<512;c++){
Engine_ engine;EngLight light{vector(),(c%17)*.5f,0};float amps[]={0,.003f,1.f/255,.01f,1,4};light.Amplify=amps[c%6];light.Position.x*=8;light.Position.y*=8;light.Position.z*=8;
sMatrix view{vector(),vector(),vector(),vector()},projection{{.05f,0,0,0},{0,.05f,0,0},{0,0,1,1},{0,0,-.125f,0}};sVector camera=vector();camera.x*=10;camera.y*=10;camera.z*=10;
if(c%8==0){camera={0,0,0,1};light.Position={0,0,(c%7)*10.f,1};}
float zoom[]={.5f+(c%7)*.25f,.75f+(c%11)*.125f};
xyz(light.Position);real(light.Range);real(light.Amplify);for(auto v:{view.i,view.j,view.k,view.l})vec(v);real(zoom[0]);real(zoom[1]);xyz(camera);for(auto v:{projection.i,projection.j,projection.k,projection.l})vec(v);
sFRect rect;CalcSphereBounds(light.Position,light.Range,rect,view,zoom);for(float f:{rect.x0,rect.y0,rect.x1,rect.y1})real(f);
engine.Env.CameraSpace.l=camera;engine.Frustum.FromViewProject(projection,{-1,-1,1,1});engine.Frustum.Normalize();engine.AddLightJob(light);emit(engine);engine.AddLightJobNonK(light);emit(engine);}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp=pathlib.Path(tmp)/'light.cpp';exe=pathlib.Path(tmp)/'light'
    cpp.write_text(support+culling.functions+normalize+sphere+'\n#define KKRIEGER 1\n'+add+'\n#undef KKRIEGER\n'+add.replace('Engine_::AddLightJob(', 'Engine_::AddLightJobNonK(')+driver)
    subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/light-visibility.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 512 original projected bounds and 1024 AddLightJob calls')
