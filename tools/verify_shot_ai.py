#!/usr/bin/env python3
"""Original ShotAI with real collision traversal and recorded damage/events."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_shot_spawn as spawn
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
oracle.source=(base/'kkriegergame.cpp').read_text(encoding='cp1252');ai=oracle.function('sBool KKriegerGame::ShotAI(')
oracle.source=(base/'_types.cpp').read_text(encoding='cp1252')
functions=spawn.functions+'\n'+spawn.table+spawn.fire+'\n'+'\n'.join(oracle.function(s) for s in ['void sMatrix::InitRot(const sVector &v)','void sMatrix::InitRot(const sVector &v,sF32 angle)','void sMatrix::Mul3(const sMatrix &a,const sMatrix &b)'])+'\n'+ai
support='#define KPK_WEAPON0 8\nint damageKind,damageHits,damageMonster;\n'+spawn.support
support=support.replace('void*Monster=nullptr;', 'KKriegerMonster*Monster=nullptr;')
support=support.replace('void InitDir(const sVector&);','void InitRot(const sVector&);void InitRot(const sVector&,float);void Mul3(const sMatrix&,const sMatrix&);void Mul3(const sMatrix&b){sMatrix a=*this;Mul3(a,b);}void InitDir(const sVector&);')
support=support.replace(' void Sub3(', ' void Add3(const sVector&a,const sVector&b){x=a.x+b.x;y=a.y+b.y;z=a.z+b.z;}void Sub3(')
support=support.replace('struct KKriegerMonster {','struct KKriegerGame;struct KKriegerMonster {void Hit(int hits,KKriegerGame*){damageKind=1;damageHits=hits;damageMonster=Id;}')
support=support.replace('KKriegerCellDynamic HitCell;KKSolidCollider Collider;', 'void Hit(int hits){damageKind=2;damageHits=hits;}KKriegerCellDynamic HitCell;KKSolidCollider Collider;')
support=support.replace('Array<KKriegerShot>Shots;', 'int WeaponHits[8];KOp*WeaponExplode[2][8];KEnvironment*Environment;bool ShotAI(KKriegerShot*);Array<KKriegerShot>Shots;')
support=support.replace('struct KEnvironment {KKriegerGame*Game;};','struct KEnvironment {KKriegerGame*Game;std::vector<KEvent*>Dynamic;void AddDynamicEvent(KEvent*e){Dynamic.push_back(e);}};')
support+='double sFCos(double x){return cos(x);}double sFSin(double x){return sin(x);}\n'
driver=r'''
void inputcell(KKriegerCell&cell,int id,int mode,float x,float z,float sx,float sy,float sz){sVector v[8];for(int k=0;k<8;k++)v[k]={x-((k&1)-0.5f)*sx,-(((k>>1)&1)-0.5f)*sy,z-(((k>>2)&1)-0.5f)*sz,1};word(mode);for(auto p:v)vector(p);cell.Init(v,mode);cell.Id=id;}
void dumpmatrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vector(v);}
int main(){word(64);for(int c=0;c<64;c++){
 KKriegerGame game;KEnvironment env{&game};game.Environment=&env;KKriegerCellAdd adds[2];KKriegerCell other[2];KKriegerCellDynamic target;KKriegerMonster monster;monster.Id=42;KOp ops[3]{};for(int i=0;i<3;i++)ops[i].Id=71+i;
 word(4);inputcell(adds[0],0,0,0,0,6,4,4);inputcell(adds[1],1,0,5,0,6,4,4);inputcell(other[0],2,1,1,1,0.5f,2,0.5f);inputcell(other[1],3,2,0,1,2,3,2);
 word(2);inputcell(game.Player.HitCell,4,2,c%4==0?10.f:0.f,0,1,2,1);word(0);inputcell(target,5,c%2?1:2,(c%4==0||((c/8)%2&&c%3==1))?10.f:1.f,1,2,2,3);target.Monster=&monster;word(42);
 game.DCellUsed=2;game.DCellPtr[0]=&game.Player.HitCell;game.DCellPtr[1]=&target;
 for(auto&cell:adds){*game.CellAdd.Add()=&cell;*game.CellList.Add()=&cell;}for(auto&cell:other)*game.CellList.Add()=&cell;game.CellConnect();
 int weapon=c%8,seed=c-32;bool ismonster=(c/8)%2;game.WeaponShot[weapon]=&ops[0];game.WeaponExplode[0][weapon]=c%3?&ops[1]:nullptr;game.WeaponExplode[1][weapon]=c%5?&ops[2]:nullptr;game.WeaponHits[weapon]=10+c;
 game.PlayerPos={0,0,0,1};game.PlayerCell=&adds[0];game.PlayerLook=0;game.PlayerDir=(c%3-1)*0.25f;game.LastCamY=0;game.CamOffset=0;
 monster.Collider.Pos=game.PlayerPos;monster.Collider.Cell=&adds[0];sVector dir={0,0,1,0};
 word(weapon);word(ismonster);word(seed);real(game.PlayerDir);word(game.WeaponHits[weapon]);word(game.WeaponExplode[0][weapon]?72:~0u);word(game.WeaponExplode[1][weapon]?73:~0u);
 sSetRndSeed(seed);game.FireShot(&env,weapon,ismonster?&monster:nullptr,&dir);word(game.Shots.Count);
 if(game.Shots.Count){auto&shot=game.Shots[0];shot.DoubleKill=c%17==0;word(shot.DoubleKill);
  for(int step=0;step<20;step++){
   damageKind=damageHits=damageMonster=0;env.Dynamic.clear();word(1);bool alive=game.ShotAI(&shot);word(alive);word(shot.Cell->Id);word(shot.DoubleKill);vector(shot.OldPos);vector(shot.Tensor);dumpmatrix(shot.Matrix);
   word(damageKind);if(damageKind){word(damageHits);if(damageKind==1)word(damageMonster);}word(env.Dynamic.size());for(auto event:env.Dynamic){word(event->Op?event->Op->Id:~0u);dumpmatrix(event->Matrix);delete event;}
   for(auto cell:game.CellList.v){word(cell->Enter);word(cell->Leave);word(cell->Collided);}for(int i=0;i<2;i++){auto cell=game.DCellPtr[i];word(cell->Enter);word(cell->Leave);word(cell->Collided);}
   if(!alive)break;
  }word(0);delete shot.Event;
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'shotai.cpp';exe=pathlib.Path(tmp)/'shotai';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/shot-ai.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 64 original projectile trajectories')
