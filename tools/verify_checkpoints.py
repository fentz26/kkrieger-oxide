#!/usr/bin/env python3
"""Original checkpoint registration, SetPlayer, RespawnAt and Continue."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_player_motion as motion
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
oracle.source=(base/'kkriegergame.cpp').read_text(encoding='cp1252')
functions=motion.functions+'\n'+'\n'.join(oracle.function(s) for s in ['void KKriegerCell::Init(const sMatrix &mat','void KKriegerCellDynamic::PrepareDynamic(','void KKriegerGame::SetPlayer(','void __stdcall Exec_KKrieger_Respawn(','void KKriegerGame::RespawnAt(','void KKriegerGame::Continue('])
support='#define __stdcall\n#define KCM_UNCONFIRMED 0x20\n#define KGS_RESPAWN 32\n#define KMS_RESPAWN 0\n'+motion.support
support=support.replace(' void Rotate34(const sMatrix&,const sVector&);',' void Rotate34(const sMatrix&,const sVector&);void Rotate34(const sMatrix&m){sVector v=*this;Rotate34(m,v);}')
support=support.replace('void Add3(const sVector&a){', 'void Add3(const sVector&a,const sVector&b){x=a.x+b.x;y=a.y+b.y;z=a.z+b.z;}void Add3(const sVector&a){')
support=support.replace('void Init(const sVector*,int);','void Init(const sMatrix&,const sVector&,int);void Init(const sVector*,int);')
support=support.replace('sMatrix Matrix0,Matrix1;', 'sMatrix Matrix0,Matrix1,ConfirmedMatrix,CurrentMatrix;sVector OrigPlanes[6],OrigVertices[8];int Blocked;void PrepareDynamic(sMatrix&);')
support=support.replace('struct KKriegerGame {','''struct KEnvironment;
struct sInputData{int Analog[2];};struct System{void GetInput(int,sInputData&id){id.Analog[0]=3;id.Analog[1]=7;}}systemObject;System*sSystem=&systemObject;
struct KKriegerRespawn{int Valid=0,Active=0;sVector Pos;float Dir=0;};
struct KKriegerMonster{int Life,State;};
struct KKriegerGame {
 KKriegerRespawn Respawn[16];unsigned char Switches[256]{};Array<KKriegerMonster*>Monsters;
 float PlayerLook,LastCamY;sVector CamPos,PlayerStartPos;int LastMouseX,LastMouseY;
 void SetPlayer(const sVector&,float,float);void RespawnAt(int);void Continue();
''')
support=support.replace('KKriegerCellDynamic HitCell;KKSolidCollider Collider;', 'int Life,Ammo[4];KKriegerCellDynamic HitCell;KKSolidCollider Collider;')
support+='struct KEnvironment{KKriegerGame*Game;};struct Triple{float x,y,z;};using sF323=Triple;\n'
driver=r'''
void dumpmatrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vector(v);}
void dump(KKriegerGame&g){vector(g.Player.Collider.Pos);vector(g.Player.Collider.OldPos);word(g.Player.Collider.Cell->Id);real(g.Player.Collider.Radius);real(g.PlayerDir);real(g.PlayerLook);real(g.LastCamY);vector(g.CamPos);dumpmatrix(g.Player.HitCell.Matrix0);dumpmatrix(g.Player.HitCell.Matrix1);word(g.JumpTimer);real(g.SpeedUp);word(g.OnGround);}
int main(){word(64);for(int c=0;c<64;c++){
 KKriegerGame game;KEnvironment env{&game};KOp op;KKriegerCellAdd add;sMatrix mat;mat.Init();sVector scale{20,20,20,0};add.Init(mat,scale,0);add.Id=0;*game.CellAdd.Add()=&add;
 game.JumpTimer=7;game.SpeedUp=0.2f;game.OnGround=1;game.PlayerMat.Init();game.PlayerStartPos={1,2,3,1};
 word(16);for(int i=0;i<16;i++){
  bool valid=(c+i)%3!=0;bool active=(c+i)%4==0;game.Respawn[i].Active=i%2;
  word(valid);word(active);word(game.Respawn[i].Active);sF323 pos{(i-7)*0.5f,(c%5-2)*0.25f,float(i%3)};float dir=(c+i)%8*0.125f;for(float f:{pos.x,pos.y,pos.z,dir})real(f);
  if(valid)Exec_KKrieger_Respawn(&op,&env,pos,i,dir);game.Switches[32+i]=active;
 }
 game.Player.Life=c;game.Player.Ammo[0]=c%2?250:5;KKriegerMonster monsters[3]={{10,4},{0,1},{-1,3}};for(auto&m:monsters)*game.Monsters.Add()=&m;
 game.Continue();dump(game);word(game.Player.Life);word(game.Player.Ammo[0]);for(auto&m:monsters)word(m.State);
 for(auto&r:game.Respawn){word(r.Valid);word(r.Active);vector(r.Pos);real(r.Dir);}
 int index=c%16;game.SoundCount=0;game.RespawnAt(index);word(index);word(game.SoundCount);dump(game);
 sVector requested{30.f,c%2?40.f:-20.f,0,1};vector(requested);game.SetPlayer(requested,0.25f,-0.5f);dump(game);
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'checkpoint.cpp';exe=pathlib.Path(tmp)/'checkpoint';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/checkpoints.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 64 original checkpoint/continue/placement cases')
