#!/usr/bin/env python3
"""Unchanged Scene_Particles execution with cached positions and local matrices."""
import pathlib, subprocess, tempfile
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
oracle.source=(base/'genscene.cpp').read_text(encoding='cp1252');execute=oracle.function('void __stdcall Exec_Scene_Particles(')
oracle.source=(base/'_types.cpp').read_text(encoding='cp1252')
random='\n'.join(oracle.function(n) for n in ['sU32 sGetRnd()','sF32 sFGetRnd()','sF32 sFGetRnd(sF32 max)','void sSetRndSeed('])
direction=oracle.function('void sMatrix::InitDir(')
support=r'''
#include <cstdio>
#include <cmath>
#include <vector>
#include <algorithm>
using sU32=unsigned;using sInt=int;using sF32=float;
#define __stdcall
#define sMOBILE 1
static unsigned sRandomSeed;
struct sF323{float x,y,z;};
struct sVector{float x=0,y=0,z=0,w=0;void Init3(float a,float b,float c){x=a;y=b;z=c;w=1;}void Init4(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}float Dot3(const sVector&v)const{return x*v.x+y*v.y+z*v.z;}void Scale3(float s){x*=s;y*=s;z*=s;}void Unit3(){Scale3(float(1./sqrt(double(Dot3(*this)))));}void UnitSafe3(){float e=Dot3(*this);if(e>1e-20f)Unit3();else Init3(1,0,0);}void Cross3(const sVector&a,const sVector&b){x=a.y*b.z-a.z*b.y;y=a.z*b.x-a.x*b.z;z=a.x*b.y-a.y*b.x;}void Sub3(const sVector&v){x-=v.x;y-=v.y;z-=v.z;}};
struct sMatrix{sVector i,j,k,l;void Init(){i={1,0,0,0};j={0,1,0,0};k={0,0,1,0};l={0,0,0,1};}void InitDir(const sVector&);};
struct KSpline{void Eval(float f,sVector&v){f=std::max(0.f,std::min(1.f,f));v={f*2.f+1.f,f*-1.f+2.f,f*4.f-1.f,0};}};
struct ScenePartMem{int Reset=1,Mode=0,Count=0,Seed=0;sF323 Rand{};sVector*Pos=nullptr,*DeleteArray=nullptr;~ScenePartMem(){delete[] DeleteArray;}};
struct KOp{};struct KEnvironment{ScenePartMem mem;struct{std::vector<sMatrix> stack;void PushMul(sMatrix m){stack.push_back(m);}void Pop(){stack.pop_back();}}ExecStack;std::vector<sMatrix>draws;template<class T>T*GetInst(KOp*){return &mem;}};
void ExecSceneInputs(KOp*,KEnvironment*e,int){e->draws.push_back(e->ExecStack.stack.back());}
double sFMod(double a,double b){return fmod(a,b);}
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vec(sVector v){for(float f:{v.x,v.y,v.z,v.w})real(f);}
'''
driver=r'''
int main(){word(64);word(8);KSpline spline;KOp op;
for(int c=0;c<64;c++){KEnvironment e;sSetRndSeed(173);
for(int k=0;k<8;k++){int mode=c,count=(c%5)*7,seed=c-32;float anims[]={0,.25f,-.25f,3.125f,-8.5f,.99f,1.f,100.f};float anim=anims[k];sF323 spread{2,.5f,-4},line{1,-2,.25f},rot{1,2,3},speed{3,2,1};if(k>=2)spread.y=1;if(k>=4)count++;if(k>=6)seed++;int hasSpline=(c%3)!=0;
for(int n:{mode,count,seed,hasSpline})word(n);for(float f:{spread.x,spread.y,spread.z,anim,line.x,line.y,line.z})real(f);
e.draws.clear();Exec_Scene_Particles(&op,&e,mode,count,seed,spread,rot,speed,anim,line,hasSpline?&spline:nullptr);e.mem.Reset=0;
word(e.mem.Count);for(int i=0;i<e.mem.Count;i++)vec(e.mem.Pos[i]);word(e.draws.size());for(auto m:e.draws)for(auto v:{m.i,m.j,m.k,m.l})vec(v);word(sGetRnd());
}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'particles.cpp';exe=pathlib.Path(tmp)/'particles'
 cpp.write_text(support+random+direction+execute+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/scene-particles.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 512 original cached particle scene executions')
