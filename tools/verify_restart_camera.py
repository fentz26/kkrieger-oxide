#!/usr/bin/env python3
"""Original Restart/GetCamera plus unchanged per-slice camera statements."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_checkpoints as checkpoints
root=oracle.root;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
functions=checkpoints.functions+'\n'+'\n'.join(oracle.function(s) for s in ['void KKriegerGame::Restart(','void KKriegerCell::DoRespawn(','void KKriegerGame::GetCamera('])
start=oracle.source.index('    LastCamY = sFade(LastCamY,PlayerPos.y,DampPlayerCam);');end=oracle.source.index('\n\n',start)
camera_step=oracle.source[start:end]
functions+='\nvoid KKriegerGame::CameraStep(){\n'+camera_step+'\n}\n'
support=checkpoints.support
support='#define KKRIEGER_SWITCHES 256\n#define sMEO_PERSPECTIVE 0\n'+support
support=support.replace('struct KEvent {','struct KInstanceMem{int Deleted=0;void DeleteChain(){Deleted++;}};\nstruct KEvent {KInstanceMem*FirstInstance=nullptr;')
support=support.replace('GenScene*Scene;KLogic Logic;', 'int Respawn=0;void DoRespawn();GenScene*Scene;KLogic Logic;')
support=support.replace('struct KKriegerGame {','struct sMaterialEnv{sMatrix CameraSpace;float ZoomX,ZoomY;int Orthogonal;};\nstruct KKriegerGame {int ResetByOp=0,CamZoomPos=0;float DampPlayerCam,CamOffset;void Restart();void GetCamera(sMaterialEnv&);void CameraStep();')
support+='\nfloat sFade(float a,float b,float f){return a+(b-a)*f;}\n'
driver=r'''
void dumpmatrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vector(v);}
void dump(KKriegerGame&g){vector(g.Player.Collider.Pos);vector(g.Player.Collider.OldPos);word(g.Player.Collider.Cell->Id);real(g.Player.Collider.Radius);real(g.PlayerDir);real(g.PlayerLook);real(g.LastCamY);vector(g.CamPos);dumpmatrix(g.Player.HitCell.Matrix0);dumpmatrix(g.Player.HitCell.Matrix1);word(g.JumpTimer);real(g.SpeedUp);word(g.OnGround);}
int main(){word(64);word(4);for(int c=0;c<64;c++){
 KKriegerGame game;KKriegerCellAdd add;sMatrix mat;mat.Init();sVector scale{20,20,20,0};add.Init(mat,scale,0);add.Id=0;*game.CellAdd.Add()=&add;
 game.JumpTimer=7;game.SpeedUp=0.2f;game.OnGround=1;game.PlayerMat.Init();game.PlayerStartPos={c%2?30.f:1.f,c%2?40.f:2.f,3,1};vector(game.PlayerStartPos);
 for(int i=0;i<256;i++){game.Switches[i]=(c+i)%7;word(game.Switches[i]);}
 game.Player.Life=c;game.Player.Ammo[0]=5;KKriegerMonster monsters[3]={{10,4},{0,1},{-1,3}};for(auto&m:monsters)*game.Monsters.Add()=&m;
 KKriegerCell zones[3];KEvent events[3];KInstanceMem instances[3];for(int i=0;i<3;i++){zones[i].Respawn=(c+i)*100;zones[i].Event[1]=i==0?nullptr:&events[i];events[i].FirstInstance=i==2?nullptr:&instances[i];*game.CellZone.Add()=&zones[i];word(zones[i].Respawn);}
 game.Restart();word(game.ResetByOp);dump(game);word(game.Player.Life);word(game.Player.Ammo[0]);for(auto&m:monsters)word(m.State);for(auto b:game.Switches)word(b);for(int i=0;i<3;i++){word(zones[i].Respawn);word(instances[i].Deleted);word(events[i].FirstInstance!=nullptr);}
 for(int step=0;step<4;step++){
  game.PlayerPos={c*0.125f,(c%9-4)*0.25f+step*0.125f,-2.f,float(step%2)};game.PlayerDir=(c%9-4)*0.125f;game.PlayerLook=(c%7-3)*0.0625f;game.DampPlayerCam=(c%5-1)*0.5f;game.CamOffset=0.85f;game.CamZoomPos=(c+step)%2;
  vector(game.PlayerPos);for(float f:{game.PlayerDir,game.PlayerLook,game.DampPlayerCam,game.CamOffset})real(f);word(game.CamZoomPos);
  game.CameraStep();real(game.LastCamY);vector(game.CamPos);sMaterialEnv env;game.GetCamera(env);dumpmatrix(env.CameraSpace);real(env.ZoomX);real(env.ZoomY);word(env.Orthogonal);
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'restart_camera.cpp';exe=pathlib.Path(tmp)/'restart_camera';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/restart-camera.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 64 restarts and 256 camera updates')
