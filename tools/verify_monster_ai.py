#!/usr/bin/env python3
"""Unchanged MonsterAI with real movement/walking and recorded attack calls."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_monster_actor as actor
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger';oracle.source=(base/'kkriegergame.cpp').read_text(encoding='cp1252')
functions=actor.functions+'\n'+oracle.function('void KKriegerGame::MonsterAI(')+'\n'+oracle.function('void KKriegerMonster::Hit(')
support=actor.support
support='#define KMS_SPAWNED 2\n#define KMS_INRANGE 3\n#define KMS_ACTIVE 4\n#define KMF_WHENPREVDEAD 1\n#define KMF_WHENMACHINELIVES 2\n#define KMF_IMMOBILE 8\n#define KCM_DISABLED 16\n'+support
support=support.replace('struct PlayerType {','struct PlayerType {int Hits=0,HitCount=0;void Hit(int h){Hits=h;HitCount++;}')
support=support.replace('struct KKriegerGame {','struct KEnvironment;struct KKriegerGame {unsigned char Switches[256]{};int FireCount=0,FireWeapon;KKriegerMonster*FireMonster;sVector FireDirection,FirePosition;KKriegerCellAdd*FireCell;void MonsterAI(KKriegerMonster*,KEnvironment*,bool,bool);void FireShot(KEnvironment*,int,KKriegerMonster*,const sVector*);')
support=support.replace('void Init(float a=0', 'void UnitAbsStub();float UnitAbs3(){float e=sqrt(double(Dot3(*this)));if(e>1e-10f)Scale3(1.f/e);else Init3(1,0,0);return e;}void Sub4(const sVector&a){x-=a.x;y-=a.y;z-=a.z;w-=a.w;}void Init(float a=0')
support+='\nvoid KKriegerGame::FireShot(KEnvironment*,int weapon,KKriegerMonster*m,const sVector*d){FireCount++;FireWeapon=weapon;FireMonster=m;FireDirection=*d;FirePosition=m->Collider.Pos;FireCell=m->Collider.Cell;}\n'
driver=actor.driver[:actor.driver.index('int main()')]
driver+=r'''
int main(){word(80);word(16);for(int c=0;c<80;c++){
 KKriegerGame game;KEnvironment env{&game};KKriegerCellAdd adds[2]{};KKriegerCell other[2]{};word(4);inputcell(adds[0],0,0,0,0,0,10,10,10);inputcell(adds[1],1,0,8,0,0,10,10,10);inputcell(other[0],2,1,3,-2,0,20,1,20);inputcell(other[1],3,1,3,0,0,0.5f,6,2);for(auto&cell:adds){*game.CellAdd.Add()=&cell;*game.CellList.Add()=&cell;}for(auto&cell:other)*game.CellList.Add()=&cell;game.CellConnect();for(auto&cell:adds)game.CreateCollisionFaces(cell);
 unsigned p[29]{};auto f=[&](int i,float value){memcpy(p+i,&value,4);};f(6,100.f+(c%3)*25.f);f(7,8);f(8,4);p[9]=c%16;p[10]=50+c;p[11]=c%11;p[12]=13;f(13,0.125f);p[14]=7;f(15,0.25f);f(16,0.2f);p[17]=123;f(18,3);f(19,1);f(20,0.75f);f(21,1.5f);f(22,0.5f);f(23,0.125f);f(24,0.25f);f(25,-0.125f);p[26]=c%5;p[27]=c%7==0?-1:5+c%3;p[28]=c;for(auto w:p)word(w);
 MonsterMem mem{};mem.Reset=1;env.Memory=&mem;game.Player.Collider.Pos={-2,0,3,1};env.Parent.Init();env.Parent.l={0,0.75f,0.5f,1};KOp op{77,p,nullptr};Exec_KKrieger_Monster(&op,&env);actor(mem.Monster);
 game.GravityPlayer=0.01f;game.StairHeight=0.55f;real(game.GravityPlayer);real(game.StairHeight);
 // Register the actor hit cell for collision queries during these AI calls.
 game.DCellUsed=1;game.DCellPtr[0]=&mem.Monster.HitCell;
 for(int k=0;k<16;k++){
  auto&m=mem.Monster;env.CurrentTime=k*10;env.TimeReset=0;
  game.Player.Collider.Pos={float((c+k)%12)-2.f,0.75f,float((c+k)%5-2),1};game.Player.Collider.Cell=c%19==0?nullptr:&adds[(c+k)%2];game.PlayerCell=game.Player.Collider.Cell;game.PlayerPos=game.Player.Collider.Pos;game.PlayerPos.y+=(c%3)*0.25f;
  if(k==0){m.State=1+c%4;m.MeeleeTimer=1;m.RangedTimer=1;if(c%13==0)m.Life=0;if(c%17==0)m.Collider.Cell=nullptr;}
  m.MagnetForce={float(c%3-1)*0.001f,0,float(k%3-1)*0.001f,0};game.Switches[m.SpawnSwitch]=(c+k)%4!=0;game.Switches[123]=c%7;
  bool machine=(c+k)%2,previous=(c+k)%3==0;
  word(env.CurrentTime);word(machine);word(previous);vector(game.PlayerPos);vector(game.Player.Collider.Pos);word(game.Player.Collider.Cell?game.Player.Collider.Cell->Id:~0u);word(m.State);word(m.Life);real(m.MeeleeTimer);real(m.RangedTimer);word(m.Collider.Cell?m.Collider.Cell->Id:~0u);vector(m.MagnetForce);word(game.Switches[m.SpawnSwitch]);word(game.Switches[123]);
  game.FireCount=0;game.Player.HitCount=0;game.MonsterAI(&m,&env,machine,previous);actor(m);word(game.Switches[123]);word(game.Player.HitCount);if(game.Player.HitCount)word(game.Player.Hits);word(game.FireCount);if(game.FireCount){word(game.FireWeapon);word(game.FireMonster==&m);vector(game.FireDirection);vector(game.FirePosition);word(game.FireCell?game.FireCell->Id:~0u);}
 }
 for(auto&cell:adds)cell.DeleteFaces();
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'ai.cpp';exe=pathlib.Path(tmp)/'ai';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/monster-ai.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 1280 original monster AI updates')
