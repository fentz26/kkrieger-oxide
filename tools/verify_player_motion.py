#!/usr/bin/env python3
"""Original OnTick player movement prefix, before Physic/Zones in each slice."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
import verify_collision_movement as movement

root=oracle.root
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
start=source.index('    sInt upforce;',source.index('  // the timesliced-loop ...'))
end=source.index('    // slice',start)
prefix='void KKriegerGame::PlayerSlice(sInt slices){sF32 d;sVector speed;KKriegerCollideInfo ci;\n'+source[start:end]+'\n}\n'
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_types.cpp').read_text(encoding='cp1252')
functions=movement.functions+'\n'+oracle.function('void sMatrix::InitEuler(sF32 a,sF32 b,sF32 c)')+'\n'+prefix
support=movement.support.replace(' void Init4(', ' void Init(float a=0,float b=0,float c=0,float d=1){x=a;y=b;z=c;w=d;}void Init4(').replace('void InitEulerPI2(const float*);','void InitEuler(float,float,float);void InitEulerPI2(const float*);')
support=support.replace('sVector Pos;float Radius;', 'sVector Pos,OldPos;float Radius;')
support=support.replace('void*Monster=nullptr;', 'void*Monster=nullptr;sMatrix Matrix0,Matrix1;')
support=support.replace('KKriegerCellDynamic HitCell;}Player;', 'KKriegerCellDynamic HitCell;KKSolidCollider Collider;}Player;')
support=support.replace(' bool MoveCollider(', '''
 float GravityPlayer,DampPlayerGround,DampPlayerAir,StairHeight,AccelSide,AccelForw,SpeedUp,OffGroundTime,StepTimer,PlayerDir;
 int JumpTimer,OnGround,StepFoot; sVector PlayerPos;KKriegerCellAdd*PlayerCell;sMatrix PlayerMat;
 int SoundCount=0;void PlayerSound(int,float=1.f){SoundCount++;}void PlayerSlice(int);
 bool MoveCollider(''')
driver=r'''
void matrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vector(v);}
void inputcell(KKriegerCell&cell,int id,int mode,float x,float sx,float sy,float sz){sVector v[8];for(int k=0;k<8;k++)v[k]={x-((k&1)-0.5f)*sx,-(((k>>1)&1)-0.5f)*sy,-(((k>>2)&1)-0.5f)*sz,1};word(mode);for(auto p:v)vector(p);cell.Init(v,mode);cell.Id=id;}
int main(){word(64);word(12);for(int c=0;c<64;c++){
 KKriegerGame game;KKriegerCellAdd adds[2];KKriegerCell other[2];word(4);
 inputcell(adds[0],0,0,0,6,4,4);inputcell(adds[1],1,0,5,6,4,4);inputcell(other[0],2,1,1.5f,0.5f,2,2);inputcell(other[1],3,2,1,3,3,3);
 for(auto&cell:adds){*game.CellAdd.Add()=&cell;*game.CellList.Add()=&cell;}for(auto&cell:other)*game.CellList.Add()=&cell;
 game.CellConnect();for(auto cell:game.CellAdd.v)game.CreateCollisionFaces(*cell);
 game.GravityPlayer=0.01f+(c%4)*0.002f;game.DampPlayerGround=0.05f;game.DampPlayerAir=0.025f;game.StairHeight=0.55f;
 for(float f:{game.GravityPlayer,game.DampPlayerGround,game.DampPlayerAir,game.StairHeight})real(f);
 auto&player=game.Player;player.Collider.Pos={-1.f,c%2?-1.4f:0.25f,0,1};player.Collider.OldPos=player.Collider.Pos;player.Collider.Cell=&adds[0];player.Collider.Radius=0.5f;
 game.PlayerPos=player.Collider.Pos;game.PlayerCell=player.Collider.Cell;vector(player.Collider.Pos);
 sVector v[8];for(int k=0;k<8;k++)v[k]={-((k&1)-0.5f)*0.75f,-0.25f-(((k>>1)&1)-0.5f)*2.5f,-(((k>>2)&1)-0.5f)*0.75f,1};player.HitCell.Init(v,2);player.HitCell.Id=4;player.HitCell.Matrix0.Init();player.HitCell.Matrix0.l=game.PlayerPos;player.HitCell.Matrix0.l.y+=0.85f;player.HitCell.Matrix1=player.HitCell.Matrix0;
 game.DCellUsed=1;game.DCellPtr[0]=&player.HitCell;
 game.JumpTimer=c%5;game.OnGround=c%2;game.OffGroundTime=c%2?0.5f:0.f;game.StepFoot=c%2;game.StepTimer=0.3f;
 word(game.JumpTimer);word(game.OnGround);real(game.OffGroundTime);word(game.StepFoot);real(game.StepTimer);
 for(int k=0;k<12;k++){
  int slices=1+(c+k)%5;game.PlayerDir=(c%9-4)*0.125f+k*0.03125f;game.PlayerMat.InitEuler(0,game.PlayerDir,0);game.AccelSide=(k%3-1)*0.005f;game.AccelForw=(k%4)*0.005f;game.SpeedUp=k==0&&c%3==0?0.1f:0.f;
  word(slices);for(float f:{game.PlayerDir,game.AccelSide,game.AccelForw,game.SpeedUp})real(f);game.SoundCount=0;game.PlayerSlice(slices);
  vector(player.Collider.Pos);vector(player.Collider.OldPos);word(player.Collider.Cell->Id);real(player.Collider.Radius);word(game.JumpTimer);word(game.OnGround);real(game.OffGroundTime);word(game.StepFoot);real(game.StepTimer);real(game.SpeedUp);word(game.SoundCount);matrix(player.HitCell.Matrix0);matrix(player.HitCell.Matrix1);
  for(auto cell:game.CellList.v){word(cell->Enter);word(cell->Leave);word(cell->Collided);}word(player.HitCell.Enter);word(player.HitCell.Leave);word(player.HitCell.Collided);
 }
 for(auto&cell:adds)cell.DeleteFaces();
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'motion.cpp';exe=pathlib.Path(tmp)/'motion';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/player-motion.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 768 original player movement slices')
