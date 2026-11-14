#!/usr/bin/env python3
"""Original Material11::Set register constants, using memory-only GPU callbacks."""
import pathlib,subprocess,tempfile
import verify_cube as cube
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
types=(base/'_types.cpp').read_text(encoding='cp1252')
support=cube.support.replace('void Init4(float a,float b,float c,float d)', '''void Scale4(const sVector&a,float f){x=a.x*f;y=a.y*f;z=a.z*f;w=a.w*f;}
 void Scale4(float f){x*=f;y*=f;z*=f;w*=f;}void Mul4(const sVector&a){x*=a.x;y*=a.y;z*=a.z;w*=a.w;}
 void AddScale4(const sVector&a,float f){x+=a.x*f;y+=a.y*f;z+=a.z*f;w+=a.w*f;}
 void Add4(const sVector&a){x+=a.x;y+=a.y;z+=a.z;w+=a.w;}void Neg3(){x=-x;y=-y;z=-z;}
 void InitColor(unsigned c){x=((c>>16)&255)/255.0f;y=((c>>8)&255)/255.0f;z=(c&255)/255.0f;w=(c>>24)/255.0f;}
 void Rotate34(const struct sMatrix&,const sVector&);void Rotate3(const struct sMatrix&);
 void Init4(float a,float b,float c,float d)''')
support=support.replace('void InitEulerPI2(const float*);','''sVector&operator[](int n){switch(n){case 0:return i;case 1:return j;case 2:return k;default:return l;}}
 const sVector&operator[](int n)const{switch(n){case 0:return i;case 1:return j;case 2:return k;default:return l;}}
 void Trans3();void Trans4();void TransR();void Mul4(const sMatrix&,const sMatrix&);void MulA(const sMatrix&,const sMatrix&);void InitEulerPI2(const float*);''')
support+='''
#define sMBF_SHADOWMASK 0x40000
#define sMBF_FOG 0x40
#define sMSF_ENVIMASK 0x30
#define sMSF_PROJMASK 0x700
#define sMSF_PROJWORLD 0x200
#define sMSF_PROJEYE 0x300
#define sMLF_MODEMASK 7
#define sMLF_OFF 0
#define sPS_00 0
#define sD3DRS_FOGCOLOR 0
#define sSwap std::swap
#define sCopyMem memcpy
template<class T>T sRange(T v,T max,T min){return std::min(max,std::max(min,v));}
struct sMaterialEnv {sMatrix ModelSpace,CameraSpace;sVector LightPos,LightColor;float LightRange,LightAmplify,FogStart,FogEnd;unsigned FogColor;};
struct sMaterialInstance {sVector*VSConstants,*PSConstants;int NumVSConstants=0,NumPSConstants=0,NumTextures;int Textures[4];};
struct System {sMatrix LastMatrix,LastViewProject,LastCamera;
 void MtrlSetSetup(int){}void SetState(int,unsigned){}
 void MtrlSetInstance(sMaterialInstance&inst){word(inst.NumVSConstants);word(inst.NumPSConstants);for(int i=0;i<inst.NumVSConstants;i++){auto&v=inst.VSConstants[i];for(float f:{v.x,v.y,v.z,v.w})real(f);}for(int i=0;i<inst.NumPSConstants;i++){auto&v=inst.PSConstants[i];for(float f:{v.x,v.y,v.z,v.w})real(f);}}
}system_instance;System*sSystem=&system_instance;
struct sMaterial11 {
 unsigned BaseFlags,SpecialFlags,LightFlags,pad1,TFlags[4],Combiner[13],pad3,pad4,AlphaCombiner;
 float TScale[4];unsigned Color[4];float SpecPower;unsigned pad5;float SRT1[9],SRT2[5];
 int ShaderLevel=1;bool HasTexSRT;int TexUse[4]{-1,-1,-1,-1},Tex[4]{-1,-1,-1,-1};int Setup=0;
 void Set(const sMaterialEnv&);
};
void matrix(sMatrix&m){for(int r=0;r<4;r++){auto&v=m[r];for(float f:{v.x,v.y,v.z,v.w})real(f);}}
'''
old=oracle.source;oracle.source=types
functions=cube.functions+'\n'
for sig in ['void sMatrix::Trans3(','void sMatrix::Trans4(','void sMatrix::TransR(','void sMatrix::Mul4(','void sMatrix::MulA(']:
 f=oracle.function(sig)
 if 'sVector *out = &i;' in f:
  f=f.replace('sVector *out = &i;','').replace('sF32 ax,ay,az,aw;','sVector *out = &(*this)[n];sF32 ax,ay,az,aw;').replace('sF32 ax,ay,az;','sVector *out = &(*this)[n];sF32 ax,ay,az;').replace('out++;','')
 functions+=f+'\n'
functions+='''void sVector::Rotate34(const sMatrix&m,const sVector&v){Scale4(m.i,v.x);AddScale4(m.j,v.y);AddScale4(m.k,v.z);Add4(m.l);}
void sVector::Rotate3(const sMatrix&m){sVector v=*this;Scale3(m.i,v.x);AddScale3(m.j,v.y);AddScale3(m.k,v.z);}
'''
oracle.source=(base/'materials/material11.cpp').read_text();functions+=oracle.function('void sMaterial11::Set(');oracle.source=old
# Source InitSRT's x86 adjacent parameter layout is already adapted in cube.functions.
driver=r'''
int main(){word(128);for(unsigned c=0;c<128;c++){
 sMaterial11 s;memset(&s.BaseFlags,0,48*4);s.BaseFlags=(c&1?0x40000:0)|(c&2?0x40:0);s.SpecialFlags=(c&4?0x10:0)|(((c>>4)%4)*0x100);s.LightFlags=c&8?5:0;
 s.HasTexSRT=c&64;if(s.HasTexSRT){s.TFlags[0]=0x2000;s.Combiner[3]=0x10;}
 for(int i=0;i<4;i++){s.TScale[i]=0.5f+i*0.375f;s.Color[i]=0x12345678u+c*0x1000100u+i*0x10305u;}
 s.SpecPower=-8.0f+c;s.SRT1[0]=1.25;s.SRT1[1]=0.75;s.SRT1[2]=1;s.SRT1[3]=0.0625;s.SRT1[4]=0.125;s.SRT1[5]=-0.03125;s.SRT1[6]=0.25;s.SRT1[7]=-0.5;s.SRT1[8]=0.75;
 s.SRT2[0]=1.25;s.SRT2[1]=0.75;s.SRT2[2]=0.25;s.SRT2[3]=-0.125;s.SRT2[4]=0.375;
 sMaterialEnv env;float model[9]={1.25f,0.75f,1.5f,0.0625f*(c%3),-0.03125f,0.125f,-2.0f,1.5f,-0.25f};env.ModelSpace.InitSRT(model);
 float camera[9]={1,1,1,0,-0.0625f,0.03125f,1.25f,-0.5f,2.0f};env.CameraSpace.InitSRT(camera);
 system_instance.LastCamera=env.CameraSpace;system_instance.LastCamera.TransR();system_instance.LastViewProject.Init();system_instance.LastViewProject.i.x=1.5f;system_instance.LastViewProject.j.y=0.75f;system_instance.LastViewProject.k.w=0.125f;system_instance.LastViewProject.l.z=-0.5f;
 env.LightPos.Init(0.5f+c*0.03125f,-1.25f,2.5f,1);env.LightColor.Init(0.25f,0.5f,0.75f,0.875f);env.LightAmplify=0.5f+c*0.0625f;env.LightRange=c%3?float(c+1):0;env.FogStart=2.5f;env.FogEnd=128;env.FogColor=0xabcdef12;
 fwrite(&s.BaseFlags,4,48,stdout);word(s.HasTexSRT?1:0);matrix(env.ModelSpace);matrix(system_instance.LastViewProject);matrix(system_instance.LastCamera);matrix(env.CameraSpace);
 for(float f:{env.LightPos.x,env.LightPos.y,env.LightPos.z,env.LightPos.w,env.LightColor.x,env.LightColor.y,env.LightColor.z,env.LightColor.w,env.LightAmplify,env.LightRange,env.FogStart,env.FogEnd})real(f);word(env.FogColor);
 s.Set(env);
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'uniforms.cpp';exe=pathlib.Path(tmp)/'uniforms'
 cpp.write_text(support+functions+cube.cube+driver)
 subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off','-fno-strict-aliasing',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/material-uniforms.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes / 128 original Set calls')
