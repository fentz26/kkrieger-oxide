#!/usr/bin/env python3
"""Original splash phase with real rays and player/monster damage functions."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_collision_movement as movement
root=oracle.root;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
functions=movement.functions+'\n'+oracle.function('sInt KKriegerGame::SplashCalc(')+'\n'+oracle.function('void KKriegerMonster::Hit(')+'\n'+oracle.function('void KKriegerPlayer::Hit(').replace('KKriegerPlayer::Hit','KKriegerGame::PlayerType::Hit')
a=oracle.source.index('  if(SplashEnable)',oracle.source.index('  // splash damage'));b=oracle.source.index('  // fire?',a)
functions+='\nvoid KKriegerGame::ApplySplash(KEnvironment*kenv){int i;KKriegerMonster*mon;'+oracle.source[a:b]+'}\n'
support='#define KMS_INRANGE 3\n#define KMS_ACTIVE 4\n'+movement.support
support=support.replace('struct KKriegerGame {','''struct KKriegerGame;struct KEnvironment;
struct KKriegerMonster {int Life,Armor,State,KillSwitch;KKSolidCollider Collider;void Hit(int,KKriegerGame*);};
struct KKriegerGame {
 unsigned char Switches[256]{};Array<KKriegerMonster*>Monsters;
 int SplashEnable,SplashHits;float SplashRange;sVector SplashPos,PlayerPos;
 int SplashCalc(const sVector&,KKriegerCellAdd*);void ApplySplash(KEnvironment*);
''')
support=support.replace('struct PlayerType {','struct PlayerType {int Life,Armor,HurtCount=0;void Hit(int);void Sound(int,float=1.f){HurtCount++;}')
support+='struct KEnvironment{KKriegerGame*Game;};\n'
driver=r'''
void inputcell(KKriegerCell&cell,int mode,float x,float sx,float sy,float sz){sVector v[8];for(int k=0;k<8;k++)v[k]={x-((k&1)-0.5f)*sx,-(((k>>1)&1)-0.5f)*sy,-(((k>>2)&1)-0.5f)*sz,1};word(mode);for(auto p:v)vector(p);cell.Init(v,mode);}
int main(){word(64);for(int c=0;c<64;c++){
 KKriegerGame game;KEnvironment env{&game};KKriegerCellAdd adds[2];KKriegerCell other[2];KKriegerMonster monsters[4];
 word(4);inputcell(adds[0],0,0,6,4,4);inputcell(adds[1],0,5,6,4,4);inputcell(other[0],1,1,0.5f,3,2);inputcell(other[1],2,0,3,3,3);
 word(1);inputcell(game.Player.HitCell,c%2?1:2,-0.5f,0.5f,2,0.5f);game.DCellUsed=1;game.DCellPtr[0]=&game.Player.HitCell;
 for(auto&cell:adds){*game.CellAdd.Add()=&cell;*game.CellList.Add()=&cell;}for(auto&cell:other)*game.CellList.Add()=&cell;game.CellConnect();
 game.SplashEnable=c%7!=0;game.SplashHits=c*5-20;float ranges[4]={0,1,3,20};game.SplashRange=ranges[(c/4)%4];game.SplashPos={c%4==2?10.f:c%4==0?-1.f:0.f,c%4==3?-1.f:0.f,0,1};
 word(game.SplashEnable);word(game.SplashHits);real(game.SplashRange);vector(game.SplashPos);
 game.PlayerPos={-1.f,0,0,1};game.Player.Life=50+c;game.Player.Armor=c%21;vector(game.PlayerPos);word(game.Player.Life);word(game.Player.Armor);
 word(4);float xs[4]={0,2,7,-12};for(int i=0;i<4;i++){auto&m=monsters[i];m.Life=20+i*3;m.Armor=i*5;m.State=3;m.KillSwitch=i%3==0?0:i%3==1?7:255;m.Collider.Pos={xs[i],0,0,1};word(m.Life);word(m.Armor);word(m.State);word(m.KillSwitch);vector(m.Collider.Pos);*game.Monsters.Add()=&m;}
 game.Switches[7]=254;game.Switches[255]=255;game.ApplySplash(&env);
 word(game.SplashEnable);word(game.SplashHits);word(game.Player.Life);word(game.Player.Armor);word(game.Player.HurtCount);for(auto&m:monsters){word(m.Life);word(m.Armor);word(m.State);}word(game.Switches[7]);word(game.Switches[255]);
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'splash.cpp';exe=pathlib.Path(tmp)/'splash';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/splash.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 64 original complete splash phases')

trigger=oracle.function('void __stdcall Exec_KKrieger_SplashDamage(')
trigger_support=r'''
#include <cstdio>
using sInt=int;using sF32=float;using sBool=bool;
#define __stdcall
struct Vec{float x=0,y=0,z=0,w=0;};struct Mat{Vec l;};
struct Stack{Mat value;Mat&Top(){return value;}};
struct SplashDamageMem{bool old=false;};struct KOp{};
struct GameType{int SplashEnable=0,SplashHits=0;float SplashRange=0;Vec SplashPos;};
struct KEnvironment{GameType*Game;Stack ExecStack;SplashDamageMem mem;template<class T>T*GetInst(KOp*){return (T*)&mem;}};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vector(Vec v){for(float f:{v.x,v.y,v.z,v.w})real(f);}
'''
trigger_driver=r'''
int main(){word(64);word(8);for(int c=0;c<64;c++){
 GameType game;KEnvironment env{&game};KOp op;float values[8]={0,1,1,0,-1,1,0,1};
 for(int k=0;k<8;k++){
  bool clear=k%3==0;if(clear)game.SplashEnable=0;int hits=10+c+k;float range=(c%5)*0.5f+k*0.125f;env.ExecStack.value.l={float(c),k*0.25f,-0.5f,1};
  word(clear);word(hits);real(range);real(values[k]);vector(env.ExecStack.value.l);
  Exec_KKrieger_SplashDamage(&op,&env,hits,range,values[k]);word(env.mem.old);word(game.SplashEnable);word(game.SplashHits);real(game.SplashRange);vector(game.SplashPos);
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'trigger.cpp';exe=pathlib.Path(tmp)/'trigger';cpp.write_text('#include <initializer_list>\n'+trigger_support+trigger+trigger_driver)
 subprocess.run(['clang++','-std=c++17','-O2',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/splash-trigger.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 512 original splash operator executions')
