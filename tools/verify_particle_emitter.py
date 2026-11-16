#!/usr/bin/env python3
"""Original PartEmitter simulation and persistent burst state, without graphics."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger';oracle.source=(base/'geneffect.cpp').read_text(encoding='cp1252');emitter=oracle.function('void __stdcall Exec_Effect_PartEmitter(')
oracle.source=(base/'_types.cpp').read_text(encoding='cp1252');random='\n'.join(oracle.function(n) for n in ['sU32 sGetRnd()','sF32 sFGetRnd()','void sSetRndSeed('])
support=r'''
#include <cstdio>
#include <cmath>
#include <vector>
using sU32=unsigned;using sInt=int;using sF32=float;
#define __stdcall
#define sINTRO 1
#define MAXPARTSLOT 16
#define sVERIFY(x) ((void)0)
#define sMOBILE 1
static unsigned sRandomSeed;const float sPI2F=6.2831853071795864769f;
struct sF324{float x,y,z,w;};struct sMatrix;
struct sVector{float x,y,z,w;void Init(float a,float b,float c,float d=1){x=a;y=b;z=c;w=d;}void Init(sF324 v){Init(v.x,v.y,v.z,v.w);}float Dot3(const sVector&v){return x*v.x+y*v.y+z*v.z;}void Unit3(){float s=float(1./sqrt(double(Dot3(*this))));x*=s;y*=s;z*=s;}void Mul3(const sVector&v){x*=v.x;y*=v.y;z*=v.z;}void Scale4(float s){x*=s;y*=s;z*=s;w*=s;}void Rotate3(const sMatrix&,const sVector&);};
struct sMatrix{sVector i,j,k,l;};void sVector::Rotate3(const sMatrix&m,const sVector&v){x=m.i.x*v.x+m.j.x*v.y+m.k.x*v.z;y=m.i.y*v.x+m.j.y*v.y+m.k.y*v.z;z=m.i.z*v.x+m.j.z*v.y+m.k.z*v.z;}
struct Particle{sVector Pos,Speed;int Time,MaxTime;};struct ParticleSystem{Particle*Array;int Max,Used;};ParticleSystem ParticleSystems[16];struct ParticleMem2{int Reset=1,BurstCount=0;};struct KOp{};
struct KEnvironment{ParticleMem2 mem;int CurrentTime,TimeDelta;struct{sMatrix Matrix;const sMatrix&Top(){return Matrix;}}ExecStack;template<class T>T*GetInst(KOp*){return &mem;}};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vec(sVector v){for(float f:{v.x,v.y,v.z,v.w})real(f);}
'''
driver=r'''
int main(){word(32);word(16);for(int c=0;c<32;c++){KEnvironment env;KOp op;int capacity=c%5*8,burst=c%3==0?0:5+c,mode=c%16,seed=c-16;float time=.5f+c*.125f,rate=100.f+c*25;sF324 speed{.25f,.5f,1.f,.125f},rpos{1,2,.5f,.25f},rspeed{.5f,.25f,1,.125f};env.ExecStack.Matrix={{1,.125f,0,0},{0,1,.25f,0},{.125f,0,1,0},{1,2,3,1}};std::vector<Particle>array(capacity);auto&system=ParticleSystems[0];system={array.data(),capacity,0};for(int n:{capacity,burst,mode,seed})word(n);real(time);real(rate);for(auto v:{speed,rpos,rspeed}){real(v.x);real(v.y);real(v.z);real(v.w);}for(auto v:{env.ExecStack.Matrix.i,env.ExecStack.Matrix.j,env.ExecStack.Matrix.k,env.ExecStack.Matrix.l})vec(v);sSetRndSeed(seed);
for(int k=0;k<16;k++){env.CurrentTime=k*13+(c%4==0?0xffffe0:0);env.TimeDelta=k%7==0?-3:13;word(env.CurrentTime);word(env.TimeDelta);Exec_Effect_PartEmitter(&op,&env,0,mode,burst,time,rate,speed,rpos,rspeed);word(env.mem.BurstCount);word(system.Used);for(int i=0;i<system.Used;i++){auto&p=system.Array[i];vec(p.Pos);vec(p.Speed);word(p.Time);word(p.MaxTime);}env.mem.Reset=0;}word(sGetRnd());}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'emitter.cpp';exe=pathlib.Path(tmp)/'emitter';cpp.write_text(support+random+emitter+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/particle-emitter.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 512 original particle emission calls')
