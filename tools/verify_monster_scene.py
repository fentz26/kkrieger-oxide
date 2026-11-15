#!/usr/bin/env python3
"""Original complete monster/flags/event/splash handlers with native instance heads."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_monster_actor as actor
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger';source=(base/'kkriegergame.cpp').read_text(encoding='cp1252');oracle.source=source
full=oracle.function('void __stdcall Exec_KKrieger_Monster(');flags=oracle.function('void __stdcall Exec_KKrieger_MonsterFlags(');splash=oracle.function('void __stdcall Exec_KKrieger_SplashDamage(')
a=source.index('struct SplashDamageMem');b=source.index('void __stdcall Exec_KKrieger_SplashDamage(',a);splash_memory=source[a:b]
oracle.source=(base/'kdoc.cpp').read_text(encoding='cp1252');exec_event=oracle.function('void KOp::ExecEvent(')
functions=actor.functions.replace(actor.init,full)+'\n'+flags+'\n'+exec_event+'\n'+splash
support=actor.support
support='#include <type_traits>\n#define KMS_INRANGE 3\n#define KV_LEG_L0 16\n#define KV_LEG_TIMES 27\n#define sUSE_TIME_OF_YEAR 0\n'+support
support=support.replace('struct KSpline;struct KOp{','struct KEnvironment;struct KEvent;struct KSpline;struct KOp{KOp*Inputs[2]{};KOp*GetInput(int i){return Inputs[i];}void ExecInputs(KEnvironment*);void ExecEvent(KEnvironment*,KEvent*,bool);void ExecWithNewMem(KEnvironment*,KInstanceMem**);')
# GetInst types are declared after the portable geometry scaffold.
support=support.replace('struct KEnvironment;struct KEvent;struct KSpline;struct KOp{','struct KInstanceMem;struct KEnvironment;struct KEvent;struct KSpline;struct KOp{')
support=support.replace('struct sVector {','struct sVector {void InitS(float f){x=y=z=w=f;}void InitColor(unsigned c){x=((c>>16)&255)/255.f;y=((c>>8)&255)/255.f;z=(c&255)/255.f;w=(c>>24)/255.f;}')
support=support.replace('struct KKriegerGame {','struct KKriegerGame {Array<KKriegerMonster*>Monsters;int SplashEnable=0,SplashHits=0;float SplashRange=0;sVector SplashPos{0,0,0,0};void AddDCell(KKriegerCellDynamic*d){DCellPtr[DCellUsed++]=d;}')
a=support.index('struct KEnvironment{');b=support.index('\n',a)
support=support[:a]+r'''
struct KEnvironment{
 KKriegerGame*Game;int CurrentTime=0,TimeReset=0,BeatTime=0,NextMonsterOverride=0,NextMonsterKillSwitch=0,NextMonsterSpawnSwitch=0,NextMonsterFlags=0;
 sMatrix Parent;struct Stack{KEnvironment*Env;std::vector<sMatrix>Frames;sMatrix&Top(){return Frames.empty()?Env->Parent:Frames.back();}void PushIdentity(){sMatrix m;m.Init();Frames.push_back(m);}void PushMul(const sMatrix&m){sMatrix result;result.MulA(m,Top());Frames.push_back(result);}void Pop(){Frames.pop_back();}}ExecStack{this};
 void*Memory;template<class T>T*GetInst(KOp*);
 sVector Var[32]{};KKriegerMonster*EventMonster=nullptr;KSpline*EventSpline=nullptr;KEvent*CurrentEvent=nullptr;int PaintCount=0,PaintInput=0;
};
'''+support[b:]
support+=splash_memory+r'''
template<class T>T*KEnvironment::GetInst(KOp*){
 if constexpr(std::is_same_v<T,MonsterMem>)return(T*)Memory;
 else{auto&head=CurrentEvent->FirstInstance;if(!head){head=new T{};head->Reset=1;}else head->Reset=0;return(T*)head;}
}
'''
helpers=r'''
void KOp::ExecInputs(KEnvironment*env){for(auto input:Inputs)if(input)Exec_KKrieger_Monster(input,env);}
void KOp::ExecWithNewMem(KEnvironment*env,KInstanceMem**){env->PaintCount++;env->PaintInput=Id;Exec_KKrieger_SplashDamage(this,env,Id==101?10:20,8,1);}
'''
driver=actor.driver[:actor.driver.index('int main()')]+r'''
int main(){word(20);word(6);for(int c=0;c<20;c++){
 KKriegerGame game;KEnvironment env{&game};KKriegerCellAdd add{};sMatrix ident;ident.Init();sVector scale{20,20,20,0};add.Init(ident,scale,0);add.Id=0;*game.CellAdd.Add()=&add;*game.CellList.Add()=&add;game.CellConnect();
 unsigned p[29]{};auto f=[&](int i,float v){memcpy(p+i,&v,4);};p[9]=c%16;p[10]=c%4==0?0:10+c;f(20,0.75f);f(21,1.5f);f(22,0.5f);p[26]=c%5;for(auto w:p)word(w);
 float srt[9]={1,1,1,0,c*0.015625f,0,c%11==0?30.f:c*0.25f,0.75f,0.5f};for(auto f:srt)real(f);env.Parent.InitSRT(srt);game.Player.Collider.Pos={-2,0,3,1};vector(game.Player.Collider.Pos);
 MonsterMem mem{};env.Memory=&mem;KOp op{};op.Id=77;op.Words=p;op.Spline=nullptr;KOp live{},death{},wrapper{};live.Id=101;death.Id=102;op.Inputs[0]=&live;op.Inputs[1]=&death;wrapper.Inputs[0]=&op;
 for(int k=0;k<6;k++){
  mem.Reset=k==0;if(k==1)mem.Monster.State=3;if(k==2)mem.Monster.State=4;if(k==3){mem.Monster.Life=0;mem.Monster.State=4;}if(k==4)mem.Monster.State=0;if(k==5)mem.Monster.State=3;
  if(k==1||k==2||k==5)mem.Monster.Event.Matrix=mem.Monster.Matrix;
  env.CurrentTime=k*10;env.PaintCount=0;env.PaintInput=0;env.Var[27]={7,8,9,10};game.Monsters.Init();game.DCellUsed=0;game.SplashEnable=0;
  Exec_KKrieger_MonsterFlags(&wrapper,&env,7,9,4);actor(mem.Monster);word(game.Monsters.Count);word(game.DCellUsed);word(env.PaintCount);word(env.PaintInput);word(game.SplashEnable);word(game.SplashHits);real(game.SplashRange);vector(game.SplashPos);word(env.NextMonsterOverride);
  for(int i=16;i<24;i++)vector(env.Var[i]);vector(env.Var[27]);word(mem.Monster.Event.FirstInstance!=nullptr);word(mem.Monster.DeathEvent.FirstInstance!=nullptr);
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'scene.cpp';exe=pathlib.Path(tmp)/'scene';cpp.write_text(support+functions+helpers+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/monster-scene.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 120 original complete monster scene executions')
