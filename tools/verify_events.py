#!/usr/bin/env python3
"""Native original event registration, expiry and scoped execution context."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kdoc.cpp').read_text(encoding='cp1252')
functions='\n'.join(oracle.function(s) for s in ['void KEnvironment::AddStaticEvent(','void KEnvironment::ExitFrame(','void KEvent::Stop(','void KOp::ExecEvent('])
support=r'''
#include <cstdio>
#include <vector>
#include <cmath>
using sInt=int;using sF32=float;using sBool=bool;
#define sLINK_KKRIEGER 0
#define sUSE_TIME_OF_YEAR 0
int deletes=0;template<class T>T sRange(T v,T hi,T lo){return v<lo?lo:v>hi?hi:v;}int sFtol(float f){return (int)std::nearbyint(f);}
struct sVector{float x=0,y=0,z=0,w=0;void InitS(float f){x=y=z=w=f;}void InitColor(unsigned c){x=((c>>16)&255)/255.f;y=((c>>8)&255)/255.f;z=(c&255)/255.f;w=(c>>24)/255.f;}};
struct KInstanceMem{void DeleteChain(){deletes++;}};struct KSpline{};struct KKriegerMonster{};struct KEnvironment;struct KEvent;
struct KOp{KEvent*FirstEvent=nullptr;void ExecEvent(KEnvironment*,KEvent*,bool);void ExecWithNewMem(KEnvironment*,KInstanceMem**);};
struct KEvent{int Id,Start=0,End=0;KOp*Op=nullptr;KEvent*NextOp=nullptr;KInstanceMem*FirstInstance=nullptr;float StartInterval=0,EndInterval=1,Velocity=0,Modulation=0;int Select=0;sVector Scale,Rotate,Translate;unsigned Color=0;KSpline*Spline=nullptr;KKriegerMonster*Monster=nullptr;int Matrix=0;void Stop();};
template<class T>struct Array{std::vector<T>v;int Count=0;T&operator[](int i){return v[i];}T*Add(){v.resize(++Count);return &v.back();}};
struct Stack{void PushMul(int){}void Pop(){}};
struct KEnvironment{int BeatTime=0,ClearInstanceMem=0;Array<KOp*>EventOpsCleanup;Array<KEvent*>DynamicEvents;sVector Var[8];KKriegerMonster*EventMonster=nullptr;KSpline*EventSpline=nullptr;KEvent*CurrentEvent=nullptr;Stack ExecStack;void AddStaticEvent(KEvent*);void ExitFrame();};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vec(sVector v){real(v.x);real(v.y);real(v.z);real(v.w);}
void KOp::ExecWithNewMem(KEnvironment*e,KInstanceMem**){for(auto v:e->Var)vec(v);word(e->EventMonster!=nullptr);word(e->EventSpline!=nullptr);word(e->CurrentEvent->Id);}
'''
driver=r'''
int main(){word(96);word(8);for(int c=0;c<96;c++){
 KEnvironment env;KOp ops[3];KEvent statics[4];KInstanceMem mem;
 for(int i=0;i<8;i++){
  KEvent*event=i<4?&statics[i]:new KEvent;event->Id=i;event->Op=&ops[(c+i)%3];event->Start=(i%4-1)*65536;event->End=i%3==0?event->Start:event->Start+(1+(c+i)%4)*65536;event->FirstInstance=&mem;
  word((c+i)%3);word(event->Start);word(event->End);if(i>=4)*env.DynamicEvents.Add()=event;
 }
 for(int tick=0;tick<8;tick++){
  env.BeatTime=(tick-2)*32768+c*257;env.ClearInstanceMem=(c+tick)%3==0;deletes=0;
  word(env.BeatTime);word(env.ClearInstanceMem);
  for(int i=0;i<env.DynamicEvents.Count;i++)env.AddStaticEvent(env.DynamicEvents[i]);for(auto&e:statics)env.AddStaticEvent(&e);
  word(deletes);for(auto&op:ops){int n=0;for(auto e=op.FirstEvent;e;e=e->NextOp)n++;word(n);for(auto e=op.FirstEvent;e;e=e->NextOp)word(e->Id);}
  deletes=0;env.ExitFrame();word(deletes);word(env.DynamicEvents.Count);for(int i=0;i<env.DynamicEvents.Count;i++)word(env.DynamicEvents[i]->Id);
 }
 for(int i=0;i<env.DynamicEvents.Count;i++)delete env.DynamicEvents[i];
}
word(128);for(int c=0;c<128;c++){
 KEnvironment env;KOp op;KEvent event;KSpline spline,oldSpline;KKriegerMonster monster,oldMonster;
 env.BeatTime=(c%17-4)*32768;event.Id=c;event.Start=(c%5-2)*65536;event.End=event.Start+(c%4)*65536;event.StartInterval=(c%3-1)*0.25f;event.EndInterval=0.25f+(c%7)*0.5f;event.Velocity=c*0.125f;event.Modulation=-c*0.0625f;event.Select=c%19;event.Scale={1.f,c*0.25f,-1.f,0};event.Rotate={-0.5f,0.25f,c*0.125f,0};event.Translate={float(c),-2.f,3.f,0};event.Color=0x11223344u*c;event.Spline=c%3?&spline:nullptr;event.Monster=c%2?&monster:nullptr;
 env.EventSpline=c%2?&oldSpline:nullptr;env.EventMonster=&oldMonster;KEvent oldEvent;env.CurrentEvent=&oldEvent;
 for(int i=0;i<8;i++)env.Var[i]={float(i),float(i+1),float(i+2),float(i+3)};
 word(env.BeatTime);word(event.Start);word(event.End);real(event.StartInterval);real(event.EndInterval);real(event.Velocity);real(event.Modulation);word(event.Select);vec(event.Scale);vec(event.Rotate);vec(event.Translate);word(event.Color);word(event.Spline!=nullptr);word(event.Monster!=nullptr);word(env.EventSpline!=nullptr);
 op.ExecEvent(&env,&event,false);
 for(auto v:env.Var)vec(v);word(env.EventMonster==&oldMonster);word(env.EventSpline==(c%2?&oldSpline:nullptr));word(env.CurrentEvent==&oldEvent);
 event.Stop();word(event.Start);word(event.End);
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'events.cpp';exe=pathlib.Path(tmp)/'events';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/events.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 768 schedules and 128 scoped contexts')
