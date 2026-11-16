#!/usr/bin/env python3
"""Original player scene camera transform with ABI-portable SRT parameters."""
import pathlib,subprocess,tempfile
import verify_scene_rotate as reference
root=reference.root;oracle=reference.oracle;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
oracle.source=(base/'_types.cpp').read_text(encoding='cp1252');functions='\n'.join(oracle.function(n) for n in ['void sMatrix::InitEulerPI2(', 'void sMatrix::InitSRT(', 'void sMatrix::MulA('])
support=reference.support.replace('using sInt=int;','using sInt=int;using sF32=float;\n#define sPLAYER 1\nconst float sPI2F=6.2831853071795864769f;float sFAbs(float f){return fabsf(f);}void sFSinCos(float f,float&s,float&c){s=float(sin(double(f)));c=float(cos(double(f)));}')
support=support.replace('void Cross4(', 'void Init3(float a,float b,float c){x=a;y=b;z=c;}void Init4(float a,float b,float c,float d){Init(a,b,c,d);}void Scale3(float f){x*=f;y*=f;z*=f;}void Cross4(')
support=support.replace('struct sMatrix{sVector i,j,k,l;};','''struct sMatrix{sVector i,j,k,l;void Init(){i={1,0,0,0};j={0,1,0,0};k={0,0,1,0};l={0,0,0,1};}const sVector&operator[](int n)const{return (&i)[n];}void InitEulerPI2(const float*);void InitSRT(const float*);void MulA(const sMatrix&,const sMatrix&);};''')
support=support.replace('struct KEnvironment{Stack ExecStack;struct{sMatrix CameraSpace;}CurrentCam;};','struct KEnvironment{Stack ExecStack;struct{sMatrix CameraSpace;}CurrentCam,GameCam;};')
oracle.source=(base/'genscene.cpp').read_text(encoding='cp1252');camera=oracle.function('void __stdcall Exec_Scene_Camera(')
camera=camera.replace('sF323 s,sF323 r,sF323 t','const float*srt').replace('&s.x','srt');functions+='\n'+camera
# Only replace stack parameter layout, whose &s.x indexing assumes original x86 ABI.
driver=r'''
int main(){word(512);for(int c=0;c<512;c++){KEnvironment env;KOp op;float srt[]={1+c*.001f,.5f,2.f,float(c%13-6)*.03125f,float(c%17-8)*.015625f,float(c%7-3)*.0625f,float(c%11),-2.f,3.f};if(c%19==0)srt[3]=srt[4]=srt[5]=1e-7f;sMatrix parent{{1,.125f,0,0},{0,1,.25f,0},{.125f,0,1,0},{3,4,5,1}};env.ExecStack.Push(parent);for(float f:srt)real(f);matrix(parent);Exec_Scene_Camera(&op,&env,srt);matrix(env.GameCam.CameraSpace);matrix(env.ExecStack.Top());word(op.Calls);}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'camera.cpp';exe=pathlib.Path(tmp)/'camera';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/scene-camera.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 512 original scene camera transforms')
