#!/usr/bin/env python3
"""Unchanged player IPP camera block, including spline override and camera modes."""
import pathlib,subprocess,tempfile
import verify_frame as reference
oracle=reference.oracle;root=reference.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
source=(base/'genoverlay.cpp').read_text(encoding='cp1252');a=source.index('      env.Init();',source.index('void __stdcall Exec_IPP_Viewport('));b=source.index('      kenv->CurrentCam = env;',a)+len('      kenv->CurrentCam = env;');block=source[a:b]
a=source.index('      sF32 off3d =',b);b=source.index('      // set viewport for culling',a);stereo=source[a:b]
a=source.index('      for(sInt pass=0;pass<nPasses;pass++)',b);b=source.index('        Engine->SetViewProject',a);passes=source[a:b]

support=reference.support[:reference.support.index('struct KEnvironment{')]
support=support.replace('struct sVector{','struct sVector{').replace('void Init(float a=0,float b=0,float c=0,float d=1)', '''void Init3(float a,float b,float c){x=a;y=b;z=c;}
void Init4(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}
void Cross3(const sVector&a,const sVector&b){x=a.y*b.z-a.z*b.y;y=a.z*b.x-a.x*b.z;z=a.x*b.y-a.y*b.x;}
void Scale3(float f){x*=f;y*=f;z*=f;}
void AddScale3(const sVector&a,float f){x+=a.x*f;y+=a.y*f;z+=a.z*f;}
void UnitSafe3(){float e=x*x+y*y+z*z;if(e>1e-20f)Scale3(float(1.0/sqrt(double(e))));else Init3(1,0,0);}
void Unit3(){Scale3(float(1.0/sqrt(double(x*x+y*y+z*z))));}
void Init(float a=0,float b=0,float c=0,float d=1)''')
support=support.replace('struct sMatrix{sVector i,j,k,l;','struct sMatrix{sVector i,j,k,l;void InitEulerPI2(const float*);')
support='#include <cmath>\n#define sPLAYER 1\n#define KV_TIME 0\n'+support+'''
float sFAbs(float f){return fabsf(f);}const float sPI2F=6.2831853071795864769f;
void sFSinCos(float f,float&s,float&c){s=float(sin(double(f)));c=float(cos(double(f)));}
struct GenSpline{sMatrix Space;float Zoom;void Eval(float,int,sMatrix&m,float&z){m=Space;z=Zoom;}};
struct KOp{GenSpline*Cache=nullptr;KOp*Input=nullptr;KOp*GetInput(int){return Input;}};
bool IntroStereo3D;struct TargetManager{int Count=0;void Add(int,int,int){Count++;}}targetManager,*RenderTargetManager=&targetManager;struct ClearSystem{void Clear(int,unsigned){}}clearSystem,*sSystem=&clearSystem;
struct KEnvironment{sMaterialEnv GameCam,CurrentCam;sVector Var[32];float Aspect;};
'''
oracle.source=(base/'_types.cpp').read_text(encoding='cp1252');functions=oracle.function('void sMatrix::InitEulerPI2(')+reference.camera
functions+='\nvoid run(KOp*parent,KEnvironment*kenv,int flags,sVector rot,sVector pos,float farclip,float nearclip,float centerx,float centery,float zoomx,float zoomy,unsigned fogc,float fogend,float fogst,float eyed,float focal){sMaterialEnv env;sMatrix mat;unsigned color=0xff000000;\n'+block+'\ncam(kenv->CurrentCam);\n'+stereo+'\nword(nPasses);word(flags);word(targetManager.Count);\n'+passes+'\ncam(kenv->CurrentCam);\n}\n}\n'
support+=reference.support[reference.support.index('void word('):reference.support.index('struct Document{')]
driver=r'''
int main(){word(1024);for(int c=0;c<1024;c++){
 unsigned flags=((c&1)?4:0)|(((c>>1)%4)<<8)|((c&8)?0x1000:0)|((c&16)?0x40:0)|((c>>5)&3)|(((c>>7)&3)<<4);float rot[3]={(c%17==0)?0.f:float(c%13-6)*0.015625f,float(c%11-5)*0.03125f,float(c%7-3)*0.0625f};if(c%19==0)for(auto&v:rot)v=0.0000001f;
 sVector pos{float(c%23-11),float(c%5-2),float(c%29-14),1};float farclip=100+c,nearclip=0.1f+c*0.001f,centerx=(c%7-3)*0.1f,centery=(c%9-4)*0.1f,zoomx=1+c*0.0005f,zoomy=0.5f+c*0.001f;unsigned fogc=0xff123456u+c;float fogend=90+c,fogst=c*.125f;
 word(3);word(flags);word(0xff000000);for(float f:rot)real(f);for(float f:{pos.x,pos.y,pos.z,farclip,nearclip,centerx,centery,zoomx,zoomy})real(f);word(fogc);real(fogend);real(fogst);
 KEnvironment e{};e.GameCam.Init();float angles[]={c*.0001f,c*.0002f,c*.0003f};e.GameCam.CameraSpace.InitEulerPI2(angles);e.GameCam.CameraSpace.l={3,4,5,2};if(c%31==0)e.GameCam.CameraSpace.k={0,1,0,0};e.GameCam.CenterX=.3f;e.GameCam.CenterY=-.2f;e.GameCam.ZoomX=1.7f;e.GameCam.ZoomY=1.9f;e.GameCam.NearClip=.25f;e.GameCam.FarClip=8192;
 cam(e.GameCam);e.Aspect=0.5f+(c%9)*.25f;real(e.Aspect);KOp parent,splineOp;GenSpline spline;spline.Space=e.GameCam.CameraSpace;spline.Space.l={-3,-4,-5,1};spline.Zoom=0.75f+c*.00025f;splineOp.Cache=&spline;bool hasSpline=c%3==1;word(hasSpline);if(hasSpline){parent.Input=&splineOp;matrix(spline.Space);real(spline.Zoom);}
 float eyed=(c%13-6)*0.125f,focal=(c%17==0)?0.f:float(c%9+1);IntroStereo3D=c%5!=0;targetManager.Count=0;real(eyed);real(focal);word(IntroStereo3D);
 run(&parent,&e,flags,{rot[0],rot[1],rot[2],0},pos,farclip,nearclip,centerx,centery,zoomx,zoomy,fogc,fogend,fogst,eyed,focal);
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'viewport.cpp';exe=pathlib.Path(tmp)/'viewport';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/viewport-camera.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 1024 original IPP viewport cameras')
