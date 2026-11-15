#!/usr/bin/env python3
"""Original InitFrame/MaterialEnv.Init and unchanged player-main timing prefix."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger';oracle.source=(base/'kdoc.cpp').read_text(encoding='cp1252');init=oracle.function('void KEnvironment::InitFrame(')
oracle.source=(base/'_start.cpp').read_text(encoding='cp1252');camera=oracle.function('void sMaterialEnv::Init(')
header=(base/'_start.hpp').read_text(encoding='cp1252');a=header.index('struct sMaterialEnv\n');b=header.index('\n};',a)+3;camera_type=header[a:b]
main=(base/'player_kkrieger/kkrieger_main.cpp').read_text(encoding='cp1252');a=main.index('    ThisTime = sSystem->GetTime() - FirstTime;');b=main.index('    Game->OnTick(Environment,max);',a);prefix=main[a:b]
support=r'''
#include <cstdio>
#include <cstring>
#include <vector>
using sInt=int;using sF32=float;using sU32=unsigned;using sU8=unsigned char;
#define sLINK_KKRIEGER 1
#define sVERIFY(x) ((void)0)
void sSetMem(void*p,int v,int n){memset(p,v,n);}
struct sVector{float x,y,z,w;void Init(float a=0,float b=0,float c=0,float d=1){x=a;y=b;z=c;w=d;}};
struct sMatrix{sVector i,j,k,l;void Init(){i={1,0,0,0};j={0,1,0,0};k={0,0,1,0};l={0,0,0,1};}};
struct KEvent{};struct KInstanceMem{};struct KKriegerMonster{};struct KSpline{};
template<class T>struct Array{std::vector<T>v;int Count=0;T operator[](int i){return v[i];}};
struct Stack{int Count=0;void PopAll(){Count=0;}};
'''+camera_type+r'''
struct KEnvironment{
 sVector Var[32],Stack[256];int StackAdr[256],Index; sMaterialEnv GameCam,CurrentCam;struct ::Stack ExecStack;KSpline*EventSpline;KKriegerMonster*EventMonster;KEvent*CurrentEvent;
 int BeatTime,LastTime,CurrentTime,TimeDelta,TimeReset,TimeSlices,TimeJitter;Array<KEvent*>DynamicEvents;Array<int>EventOpsCleanup;KInstanceMem*NextInstance,*DefaultInstance,**LinkInstance;
 int NextMonsterOverride,NextMonsterKillSwitch,NextMonsterSpawnSwitch,NextMonsterFlags,Added=0;
 void AddStaticEvent(KEvent*){Added++;}void InitFrame(int,int);
};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vec(sVector v){real(v.x);real(v.y);real(v.z);real(v.w);}void matrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vec(v);}
void cam(sMaterialEnv&e){vec(e.LightPos);vec(e.LightColor);real(e.LightRange);real(e.LightAmplify);real(e.FogStart);real(e.FogEnd);word(e.FogColor);matrix(e.ModelSpace);matrix(e.CameraSpace);for(float f:{e.ZoomX,e.ZoomY,e.CenterX,e.CenterY,e.NearClip,e.FarClip})real(f);word(e.Orthogonal);}
struct Document{int SongBPM;}document,*Document=&document;
struct System{int Time;int GetTime(){return Time;}}systemObject,*sSystem=&systemObject;
int sMulDiv(int a,int b,int c){return (int)((long long)a*b/c);}void sFloatFix(){}
'''
driver=r'''
int main(){word(64);word(12);for(int c=0;c<64;c++){
 KEnvironment env{};env.CurrentTime=c*13-300;word(env.CurrentTime);KInstanceMem mem;env.DefaultInstance=c%2?&mem:nullptr;word(env.DefaultInstance!=nullptr);KEvent event;env.DynamicEvents.Count=c%4;env.DynamicEvents.v.assign(c%4,&event);
 for(int k=0;k<12;k++){
  int delta[]={0,9,1,11,-5,1000,-1200,5,10,19,-30,7};int time=env.CurrentTime+delta[k],beat=(c-32)*65536+k*313;word(time);word(beat);
  memset(env.Var,0x3f,sizeof(env.Var));env.Index=99;env.EventSpline=(KSpline*)1;env.EventMonster=(KKriegerMonster*)1;env.CurrentEvent=&event;env.NextMonsterOverride=env.NextMonsterKillSwitch=env.NextMonsterSpawnSwitch=env.NextMonsterFlags=77;env.ExecStack.Count=3;env.Added=0;
  env.InitFrame(beat,time);
  for(auto v:env.Var)vec(v);for(auto v:{env.BeatTime,env.LastTime,env.CurrentTime,env.TimeDelta,env.TimeReset,env.TimeSlices,env.TimeJitter,env.Index,env.ExecStack.Count})word(v);
  word(env.EventSpline==nullptr);word(env.EventMonster==nullptr);word(env.CurrentEvent==nullptr);word(env.NextInstance==env.DefaultInstance);word(env.LinkInstance==nullptr);for(auto v:{env.NextMonsterOverride,env.NextMonsterKillSwitch,env.NextMonsterSpawnSwitch,env.NextMonsterFlags,env.Added})word(v);cam(env.GameCam);cam(env.CurrentCam);
 }
}
word(32);word(8);for(int c=0;c<32;c++){
 int FirstTime=500+c*13,LastTime=0,ThisTime,beat,max;float curfps,oldfps=0,GlobalFps;document.SongBPM=(100+c)*65536;word(FirstTime);word(document.SongBPM);
 for(int k=0;k<8;k++){
  int offsets[]={0,9,11,1000,1003,995,2000,2011};systemObject.Time=FirstTime+offsets[k];word(systemObject.Time);
'''+prefix+r'''
  LastTime=ThisTime;
  word(ThisTime);word(beat);word(max);real(curfps);real(oldfps);real(GlobalFps);word(LastTime);
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'frame.cpp';exe=pathlib.Path(tmp)/'frame';cpp.write_text(support+camera+init+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/frame.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 768 original frame initializations and 256 main-loop timing updates')
