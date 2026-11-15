#!/usr/bin/env python3
"""Original ShotInfoTable and complete FireShot initialization."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_player_motion as motion
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
game=(base/'kkriegergame.cpp').read_text(encoding='cp1252');types=(base/'_types.cpp').read_text(encoding='cp1252')
oracle.source=game;fire=oracle.function('void KKriegerGame::FireShot(')
table=game[game.index('struct ShotInfo\n'):game.index('void KKriegerGame::FireShot(')]
oracle.source=types
functions=motion.functions+'\n'+'\n'.join(oracle.function(s) for s in ['sU32 sGetRnd()','sF32 sFGetRnd()','void sSetRndSeed(','void sVector::InitRnd(','void sMatrix::InitDir('])
support=motion.support.replace(' void Sub3(', '''
 void InitRnd();void UnitSafe3(){float e=Dot3(*this);if(e>1e-20f)Scale3(float(1.0/sqrt(double(e))));else Init3(1,0,0);}
 void Rotate3(const sMatrix&m,const sVector&v);void Rotate3(const sMatrix&m){sVector v=*this;Rotate3(m,v);}
 void Scale4(float f){x*=f;y*=f;z*=f;w*=f;}
 void Sub3(''')
support=support.replace('void InitEuler(float,float,float);','void InitDir(const sVector&);void InitEuler(float,float,float);')
support=support.replace('struct KEvent {KOp*Op=nullptr;', 'struct KKriegerMonster;struct KEvent {sMatrix Matrix;int Active=0;KKriegerMonster*Monster=nullptr;KOp*Op=nullptr;')
support=support.replace('struct KKriegerGame {', '''
struct KKriegerMonster {KKSolidCollider Collider;int Id;};
struct KKriegerShot {int Weapon,Mode,Who,DoubleKill;KEvent*Event;KKriegerCellAdd*Cell;sVector OldPos,Tensor;sMatrix Matrix;};
struct KKriegerGame {
 Array<KKriegerShot>Shots;KOp*WeaponShot[8];float PlayerLook,LastCamY,CamOffset;
 void FireShot(KEnvironment*,int,KKriegerMonster*,const sVector*);
''')
# KEnvironment is a memory-only back-reference used by the original function.
support=support.replace('struct KKriegerGame {','struct KEnvironment;struct KKriegerGame {',1)
support+='''
#define sMOBILE 1
static unsigned sRandomSeed=0x74382381;
struct KEnvironment {KKriegerGame*Game;};
void sVector::Rotate3(const sMatrix&m,const sVector&v){x=m.i.x*v.x;y=m.i.y*v.x;z=m.i.z*v.x;x+=m.j.x*v.y;y+=m.j.y*v.y;z+=m.j.z*v.y;x+=m.k.x*v.z;y+=m.k.y*v.z;z+=m.k.z*v.z;}
'''
driver=r'''
void dumpmatrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vector(v);}
int main(){word(8);for(auto d:ShotInfoTable){word(d.Mode);for(float f:{d.Speed,d.rx,d.ry,d.rz,d.px,d.py,d.pz,d.sx,d.sy,d.sz})real(f);}word(128);
 for(int c=0;c<128;c++){
  KKriegerGame game;KEnvironment env{&game};KOp target{};target.Id=71;KKriegerCellAdd cell;cell.Id=17;KKriegerMonster monster;monster.Id=42;
  int weapon=c%8;bool ismonster=(c/8)%2;game.WeaponShot[weapon]=c%3?&target:nullptr;
  game.PlayerPos={float(c%7),float(c%5)*0.25f,float(c%11)*-0.125f,1};game.PlayerCell=&cell;game.PlayerLook=(c%9-4)*0.0625f;game.PlayerDir=(c%13-6)*0.125f;game.LastCamY=0.75f;game.CamOffset=1.05f;
  monster.Collider.Pos=game.PlayerPos;monster.Collider.Cell=&cell;sVector dir={float(c%3-1),float(c%5-2)*0.5f,float(c%7-3)*0.25f,0};if(c%11==0)dir={0,0,0,0};
  int seed=c-64;word(weapon);word(ismonster);word(seed);word(game.WeaponShot[weapon]?71:~0u);vector(game.PlayerPos);for(float f:{game.PlayerLook,game.PlayerDir,game.LastCamY,game.CamOffset})real(f);vector(dir);
  sSetRndSeed(seed);game.FireShot(&env,weapon,ismonster?&monster:nullptr,&dir);word(game.Shots.Count);
  if(game.Shots.Count){auto&s=game.Shots[0];word(s.Weapon);word(s.Mode);word(s.Who);word(s.DoubleKill);word(s.Cell->Id);vector(s.OldPos);vector(s.Tensor);dumpmatrix(s.Matrix);dumpmatrix(s.Event->Matrix);word(s.Event->Active);word(s.Event->Op?s.Event->Op->Id:~0u);word(s.Event->Monster?s.Event->Monster->Id:~0u);delete s.Event;}
  word(sGetRnd());
 }
}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'shots.cpp';exe=pathlib.Path(tmp)/'shots';cpp.write_text(support+functions+table+fire+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/shot-spawn.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 128 original shot initializations')
