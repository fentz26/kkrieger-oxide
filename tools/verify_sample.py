#!/usr/bin/env python3
"""Original PlaySample calls with platform sound and persistent state captured."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kdoc.cpp').read_text(encoding='cp1252');function=oracle.function('void __stdcall Exec_Misc_PlaySample(')
support=r'''
#include <cstdio>
#include <cmath>
#include <vector>
using sInt=int;using sF32=float;
#define __stdcall
#define sLINK_KKRIEGER 1
#define sPLAYER 1
double sFPow(double a,double b){return pow(a,b);}int sFtol(float v){return int(nearbyintf(v));}
struct sVector{float x=0,y=0,z=0,w=0;void Init4(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}void Sub3(const sVector&a,const sVector&b){x=a.x-b.x;y=a.y-b.y;z=a.z-b.z;}void Scale3(float f){x*=f;y*=f;z*=f;}};
struct sMatrix{sVector l;};struct KOp;struct PlaySampleMem{int Reset=1,On=0,Retrigger=0,OldValue=0;sVector OldPos;int Buffer3D=-1;};
struct Game{unsigned char Switches[256];};struct KEnvironment{PlaySampleMem mem;Game*Game;int TimeDelta;struct{sMatrix matrix;const sMatrix&Top(){return matrix;}}ExecStack;template<class T>T*GetInst(KOp*){return &mem;}};
struct KOp{int Inputs=0;void ExecInputs(KEnvironment*){Inputs++;}};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vector(sVector v){for(float f:{v.x,v.y,v.z,v.w})real(f);}
struct System{int Plays=0,Spatial=0,Next=0;std::vector<unsigned>records;int SamplePlay(int sample,float volume,int){Plays++;word(sample);real(volume);int id=(Next++%5==0)?-1:Next;word(id);return id;}void Sample3DParam(int sample,int buffer,sVector pos,sVector velocity,float near,float far){Spatial++;word(sample);word(buffer);vector(pos);real(velocity.x);real(velocity.y);real(velocity.z);real(near);real(far);}}systemObject,*sSystem=&systemObject;
'''
driver=r'''
int main(){word(32);word(64);for(int c=0;c<32;c++){Game game{};KEnvironment env;env.Game=&game;KOp op;systemObject.Next=0;for(int k=0;k<64;k++){int sw=3,val=c%3==0?-1:c%3,sample=c%12;float threshold=k%7==0?0.f:1.f,volume=float(c%17-4),retrigger=c%4==0?0.f:float(c%7)*.0125f,halfrange=k%9==0?0.f:float(c%5);int delta=k%11==0?0:10+k%5;game.Switches[sw]=(k/4)%3;env.TimeDelta=delta;env.ExecStack.matrix.l={float(k)*.25f,float(c)*.5f,float(k%7),1};word(val);word(game.Switches[sw]);real(threshold);word(sample);real(volume);real(retrigger);real(halfrange);word(delta);vector(env.ExecStack.matrix.l);
int active=val==-1?game.Switches[sw]!=(env.mem.Reset?game.Switches[sw]:env.mem.OldValue):(game.Switches[sw]==val&&threshold>0);int timer=env.mem.Reset?-1:env.mem.Retrigger;bool on=env.mem.On;if(timer>=0&&timer-delta<0)on=false;bool play=active&&!on;word(play);int before=systemObject.Next;bool spatial=(halfrange!=0&&play)?(before%5!=0):env.mem.Buffer3D!=-1;word(spatial);
systemObject.Plays=systemObject.Spatial=0;Exec_Misc_PlaySample(&op,&env,sw,val,threshold,sample,volume,retrigger,halfrange);word(systemObject.Plays);word(systemObject.Spatial);word(env.mem.On);word(env.mem.Retrigger);word(env.mem.OldValue);vector(env.mem.OldPos);word(env.mem.Buffer3D);word(op.Inputs);env.mem.Reset=0;
}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'sample.cpp';exe=pathlib.Path(tmp)/'sample';cpp.write_text(support+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/sample.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 2048 original sample handler calls')
