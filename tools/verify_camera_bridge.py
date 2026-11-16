#!/usr/bin/env python3
"""Original camera TransR and Mul4 pipeline before material uniform generation."""
import pathlib,subprocess,tempfile
import verify_projection as reference
root=reference.root;oracle=reference.oracle;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_types.cpp').read_text(encoding='cp1252')
functions=reference.reference.camera+reference.function+'\n'+'\n'.join(oracle.function(n) for n in ['void sMatrix::Trans3(', 'void sMatrix::TransR(', 'void sMatrix::Mul4('])
support=reference.support.replace('struct sVector{','struct sMatrix;\nstruct sVector{').replace('void Init(float a=0', 'void Neg3(){x=-x;y=-y;z=-z;}void Rotate3(const sMatrix&);\nvoid Init(float a=0')
support=support.replace('struct sMatrix{sVector i,j,k,l;','struct sMatrix{sVector i,j,k,l;void Trans3();void TransR();void Mul4(const sMatrix&,const sMatrix&);const sVector&operator[](int n)const{return (&i)[n];}')
support+='''\ntemplate<class T>void sSwap(T&a,T&b){T temp=a;a=b;b=temp;}
void sVector::Rotate3(const sMatrix&m){sVector v=*this;x=m.i.x*v.x+m.j.x*v.y+m.k.x*v.z;y=m.i.y*v.x+m.j.y*v.y+m.k.y*v.z;z=m.i.z*v.x+m.j.z*v.y+m.k.z*v.z;}\n'''
driver=r'''
int main(){word(256);for(int c=0;c<256;c++){sMaterialEnv e;e.Init();e.Orthogonal=c%2;e.ZoomX=.5f+c*.001f;e.ZoomY=.75f+c*.002f;e.CenterX=float(c%11-5)*.125f;e.CenterY=float(c%7-3)*.0625f;e.NearClip=.125f;e.FarClip=4096;systemObject.ViewportX=640+c;systemObject.ViewportY=360+c;e.CameraSpace={{1,.125f,0,0},{0,1,.25f,0},{.125f,0,1,0},{float(c%13-6),float(c%17-8),float(c%7-3),1}};word(e.Orthogonal);for(float f:{e.ZoomX,e.ZoomY,e.CenterX,e.CenterY,e.NearClip,e.FarClip})real(f);word(systemObject.ViewportX);word(systemObject.ViewportY);matrix(e.CameraSpace);sMatrix view=e.CameraSpace,projection,result;view.TransR();e.MakeProjectionMatrix(projection);result.Mul4(view,projection);matrix(view);matrix(result);}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'bridge.cpp';exe=pathlib.Path(tmp)/'bridge';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/camera-bridge.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 256 original camera uniform transforms')
