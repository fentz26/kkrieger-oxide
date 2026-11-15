#!/usr/bin/env python3
"""Unchanged monster initialization branch with native walking and collision."""
import pathlib,re,subprocess,tempfile
import verify_mesh as oracle
import verify_monster_walk as walk
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger';source=(base/'kkriegergame.cpp').read_text(encoding='cp1252');header=(base/'kkriegergame.hpp').read_text(encoding='cp1252');oracle.source=source
init=oracle.function('void __stdcall Exec_KKrieger_Monster(');init=init[:init.index('  sVERIFY(op->GetInput(0));')]+'}\n'
functions=walk.motion.functions+'\n'+'\n'.join(oracle.function(s) for s in ['void KKriegerCell::Init(const sMatrix &mat','void KKriegerCellDynamic::PrepareDynamic('])+walk.splines+walk.walk+init
start=header.index('struct KKriegerMonster                      //');end=header.index('\n};',start)+3;monster=header[start:end]
support=walk.support
support='#include <cstddef>\n#define __stdcall\n#define KMS_RESPAWN 0\n#define KMS_IDLE 1\n#define KCM_UNCONFIRMED 0x20\n'+support
support=support.replace('struct KOp {int Id;};','struct KSpline;struct KOp{int Id;unsigned*Words;KSpline*Spline;float*GetAnimPtrF(int){return(float*)Words;}int*GetAnimPtrS(int){return(int*)Words;}KSpline*GetSpline(int){return Spline;}};')
support=re.sub(r'struct KEvent \{[^\n]*',r'''struct KInstanceMem{int Reset;KInstanceMem**DeleteChild;KInstanceMem**DeleteChild2;};
struct KKriegerMonster;
struct KEvent{sMatrix Matrix;KOp*Op;KInstanceMem*FirstInstance;KKriegerMonster*Monster;sVector Scale,Rotate,Translate;float Velocity,Modulation,StartInterval,EndInterval;int Select,Start,End,CullDist;unsigned Color;KSpline*Spline;void Init(){memset(this,0,sizeof(*this));Matrix.Init();EndInterval=1;}};''',support,count=1)
support=support.replace('void Init(const sVector*,int);','void Init(const sMatrix&,const sVector&,int);void Init(const sVector*,int);')
support=support.replace('sMatrix Matrix0,Matrix1;', 'sMatrix Matrix0,Matrix1,ConfirmedMatrix,CurrentMatrix;sVector OrigPlanes[6],OrigVertices[8];int Blocked;void PrepareDynamic(sMatrix&);')
support=support.replace('struct KEnvironment{KKriegerGame*Game;int CurrentTime=0,TimeReset=0;};', 'struct KEnvironment{KKriegerGame*Game;int CurrentTime=0,TimeReset=0,NextMonsterOverride=0,NextMonsterKillSwitch=0,NextMonsterSpawnSwitch=0,NextMonsterFlags=0;void*Memory;template<class T>T*GetInst(KOp*){return(T*)Memory;}};')
support+='\nvoid sCopyMem(void*d,const void*s,int n){memcpy(d,s,n);}\n'+walk.memory+walk.definitions+monster+'\nstruct MonsterMem:KInstanceMem{KKriegerMonster Monster;};\n'
support+='static_assert(offsetof(KKriegerMonster,Event)-offsetof(KKriegerMonster,Speed)==56,"original parameter layout");\n'
driver=walk.driver[:walk.driver.index('int main()')]
driver+=r'''
void event(KEvent&e,KKriegerMonster*m){matrix(e.Matrix);word(e.Op?e.Op->Id:~0u);word(e.Monster==m);for(auto v:{e.Scale,e.Rotate,e.Translate})vector(v);for(float f:{e.Velocity,e.Modulation,e.StartInterval,e.EndInterval})real(f);word(e.Select);word(e.Start);word(e.End);word(e.CullDist);word(e.Color);word(e.Spline!=nullptr);}
void actor(KKriegerMonster&m){matrix(m.Matrix);for(int i=0;i<14;i++)word(((unsigned*)&m.Speed)[i]);event(m.Event,&m);event(m.DeathEvent,&m);word(m.Life);real(m.MeeleeTimer);real(m.RangedTimer);word(m.State);word(m.WalkStyle);word(m.WeaponKind);word(m.Spline!=nullptr);vector(m.MagnetForce);vector(m.PosOffset);vector(m.Collider.Pos);vector(m.Collider.OldPos);word(m.Collider.Cell?m.Collider.Cell->Id:~0u);real(m.Collider.Radius);auto&h=m.HitCell;word(h.Mode);word(h.Monster==&m);word(h.Blocked);word(h.PlaneCount);for(int i=0;i<h.PlaneCount;i++)vector(h.Planes[i]);for(auto v:h.Vertices)vector(v);matrix(h.Matrix0);matrix(h.Matrix1);matrix(h.ConfirmedMatrix);matrix(h.CurrentMatrix);dump(m.WalkMem);word(m.SpawnSwitch);}
int main(){word(40);word(3);for(int c=0;c<40;c++){
 KKriegerGame game;KEnvironment env{&game};KKriegerCellAdd adds[2]{};KKriegerCell floor{};word(3);inputcell(adds[0],0,0,0,0,0,10,10,10);inputcell(adds[1],1,0,8,0,0,10,10,10);inputcell(floor,2,1,3,-2,0,20,1,20);for(auto&cell:adds){*game.CellAdd.Add()=&cell;*game.CellList.Add()=&cell;}*game.CellList.Add()=&floor;game.CellConnect();
 unsigned p[29]{};auto f=[&](int i,float value){memcpy(p+i,&value,4);};f(6,0.1f*(c+1));f(7,8);f(8,4);p[9]=c%16;p[10]=c%7==0?0:50+c;p[11]=c%11;p[12]=13;f(13,0.75f);p[14]=7;f(15,1.25f);f(16,0.2f);p[17]=123;f(18,3);f(19,1);f(20,0.75f);f(21,1.5f);f(22,0.5f);f(23,0.125f);f(24,0.25f);f(25,-0.125f);p[26]=c%5;p[27]=5+c%3;p[28]=c;for(auto w:p)word(w);
 MonsterMem mem{};env.Memory=&mem;KSpline curve;curve.Count=4;curve.Interpolation=c%3;KSplineKey keys[4][3]={{{0,0},{0.5f,0.125f},{1,0}},{{0,0},{0.5f,0.25f},{1,0}},{{0,0},{0.5f,-0.0625f},{1,0}},{{0,0},{0.5f,1},{1,2}}};for(int i=0;i<4;i++)curve.Channel[i]={3,keys[i]};KOp op{77,p,c%2?&curve:nullptr};word(op.Spline!=nullptr);
 for(int k=0;k<3;k++){
  mem.Reset=k==0;if(k)mem.Monster.State=KMS_RESPAWN;mem.Monster.MeeleeTimer=k*0.125f;mem.Monster.RangedTimer=k*0.25f;mem.Monster.MagnetForce={k*0.1f,-0.25f,0.125f,0};
  env.CurrentTime=k*100;env.TimeReset=0;env.NextMonsterOverride=(c+k)%2;env.NextMonsterKillSwitch=100+c;env.NextMonsterSpawnSwitch=30+c;env.NextMonsterFlags=4;
  game.Player.Collider.Pos={-2,0,3,1};sMatrix parent;parent.InitEuler(0,c*0.125f,0);parent.l={c%9==0?30.f:float(k),0.75f,0.5f,1};
  word(env.CurrentTime);word(env.NextMonsterOverride);word(env.NextMonsterKillSwitch);word(env.NextMonsterSpawnSwitch);word(env.NextMonsterFlags);vector(game.Player.Collider.Pos);matrix(parent);
  // ExecStack is represented by the exact authored parent, not an engine stub.
  env.Parent=parent;
  Exec_KKrieger_Monster(&op,&env);actor(mem.Monster);word(env.NextMonsterOverride);
 }
}}
'''
# The branch only observes ExecStack.Top; retain that matrix exactly.
support=support.replace('void*Memory;template<class T>', 'sMatrix Parent;struct Stack{KEnvironment*Env;sMatrix&Top(){return Env->Parent;}}ExecStack{this};void*Memory;template<class T>')
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'actor.cpp';exe=pathlib.Path(tmp)/'actor';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/monster-actor.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 120 original monster initializations')
