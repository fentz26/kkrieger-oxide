#!/usr/bin/env python3
"""Unchanged original Misc_Trigger with persistent instance/event recording."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_events as events
root=oracle.root;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kdoc.cpp').read_text(encoding='cp1252')
function=oracle.function('void __stdcall Exec_Misc_Trigger(')
support=events.support
support=support.replace('using sInt=int;', '#define __stdcall\nusing sU32=unsigned;using sInt=int;')
support=support.replace('struct KInstanceMem{','struct KInstanceMem{bool Reset=true;')
support=support.replace('struct KSpline{};', 'struct KSpline{};struct sF323{float x,y,z;};')
support=support.replace('void InitS(float f)', 'void Init(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}void InitS(float f)')
support=support.replace('struct KOp{', 'struct TriggerMem:KInstanceMem{int Active=0,Time=0;};\nvoid Exec_Misc_Event(KOp*,KEnvironment*,float);\nstruct KOp{')
# KOp needs a forward declaration before the function prototype.
support=support.replace('void Exec_Misc_Event(KOp*','struct KOp;void Exec_Misc_Event(KOp*',1)
support=support.replace('KEvent*FirstEvent=nullptr;', 'int Id=0;KOp*Link=nullptr;void(*ExecHandler)(KOp*,KEnvironment*,float)=nullptr;KOp*GetLink(int){return Link;}void ExecInput(KEnvironment*,int);KEvent*FirstEvent=nullptr;')
support=support.replace('int Id,Start=0,End=0;', 'int Id=0,Start=0,End=0,Dynamic=0;')
support=support.replace('void Stop();', 'void Init(){*this=KEvent{};}void Stop();')
support=support.replace('struct Stack{', 'struct Matrix{ sVector l;};struct Stack{Matrix M;Matrix&Top(){return M;}')
support=support.replace('int BeatTime=0,ClearInstanceMem=0;', 'int BeatTime=0,ClearInstanceMem=0,Calls=0,Children=0;TriggerMem Mem;std::vector<KEvent*>Created;template<class T>T*GetInst(KOp*){Mem.Reset=Calls++==0;return (T*)&Mem;}void AddDynamicEvent(KEvent*e){Created.push_back(e);}')
support+='\nvoid Exec_Misc_Event(KOp*,KEnvironment*,float){}\nvoid KOp::ExecInput(KEnvironment*e,int){e->Children++;}\n'
driver=r'''
int main(){word(128);word(16);for(int c=0;c<128;c++){
 KEnvironment env;KOp op,target;target.Id=4;target.ExecHandler=c%5==0?nullptr:Exec_Misc_Event;op.Link=c%7==0?nullptr:&target;word(op.Link!=nullptr && target.ExecHandler==Exec_Misc_Event);word(c%8);
 for(int k=0;k<16;k++){
  env.BeatTime=(c%3==0?16777216:0)+(k-3)*12345+c*17;float trigger=float((c+k*3)%7-3)*0.25f,threshold=(c%3-1)*0.25f,delay=(c%5-1)*0.06251f,duration=(c%7-1)*0.12501f;int flags=c%8;
  sF323 scale{1.f,c*0.25f,-2.f},rotate{-0.25f,k*0.03125f,0.5f},translate{float(k),2.f,-3.f};env.ExecStack.M.l={float(c),float(k),-5.f,1};
  float vel=c*0.125f,mod=-k*0.0625f;int sel=c%19;unsigned col=0x12345678;KSpline spline;
  word(env.BeatTime);for(float f:{trigger,threshold,delay,duration,vel,mod})real(f);word(sel);for(auto v:{scale,rotate,translate}){real(v.x);real(v.y);real(v.z);}vec(env.ExecStack.M.l);
  Exec_Misc_Trigger(&op,&env,trigger,threshold,delay,duration,flags,vel,mod,sel,scale,rotate,translate,col,&spline);
  word(env.Mem.Active);word(env.Mem.Time);word(env.Calls);word(env.Children);word(env.Created.size());
  for(auto event:env.Created){word(event->Op->Id);word(event->Start);word(event->End);word(event->Dynamic);real(event->Velocity);real(event->Modulation);word(event->Select);vec(event->Scale);vec(event->Rotate);vec(event->Translate);word(event->Color);word(event->Spline!=nullptr);real(event->StartInterval);real(event->EndInterval);delete event;}
  env.Created.clear();
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'trigger.cpp';exe=pathlib.Path(tmp)/'trigger';cpp.write_text(support+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/trigger.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 2048 original trigger executions')
