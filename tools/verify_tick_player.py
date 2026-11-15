#!/usr/bin/env python3
"""Whole original OnTick with empty actor/shot/zone lists and real player physics."""
import pathlib,re,subprocess,tempfile
import verify_mesh as oracle
import verify_restart_camera as restart
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger';oracle.source=(base/'kkriegergame.cpp').read_text(encoding='cp1252')
functions=restart.functions+'\n'+'\n'.join(oracle.function(s) for s in ['void KKriegerGame::OnTick(','void KKriegerGame::Physic(','void KKriegerCellDynamic::ApplyMatrix('])
oracle.source=(base/'_types.cpp').read_text(encoding='cp1252');functions+='\n'+oracle.function('void sMatrix::Lin4(')
support=restart.support
support='#define KGS_GAME 1\n#define KGS_GAME_RUN 0\n#define KGS_GAME_OPTIONS 9\n#define KGS_GAME_INGAME 1\n#define KGS_INGAME_MENU 16\n#define KGS_MOUSESPEED 13\n#define KGS_MOUSEINVERT 12\n#define KMS_ACTIVE 4\n#define KMF_MACHINE 4\n#define KWF_CONTINUOUS 1\n'+support
support=support.replace('struct KKriegerMonster{int Life,State;};','struct KKriegerGame;struct KKriegerMonster{int Life,State,Flags;struct{sVector Pos;}Collider;void Hit(int,KKriegerGame*){}};')
support=support.replace('struct KKriegerMonster{','struct KKriegerMonster{')
# The checkpoint scaffold spells its tiny actor type with a space.
support=support.replace('struct KKriegerMonster{int Life,State;};','struct KKriegerGame;struct KKriegerMonster{int Life,State,Flags;struct{sVector Pos;}Collider;void Hit(int,KKriegerGame*){}};')
support=support.replace('struct KKriegerMonster {int Life,State;};','struct KKriegerGame;struct KKriegerMonster {int Life,State,Flags;struct{sVector Pos;}Collider;void Hit(int,KKriegerGame*){}};')
a=support.index('struct System{');b=support.index('System*sSystem=&systemObject;',a)+len('System*sSystem=&systemObject;')
support=support[:a]+r'''struct Listener{sVector Pos,Speed,Up,Forward;};static std::vector<Listener>listeners;
struct System{int Time=0,Mouse[2]{};int GetTime(){return Time;}void GetInput(int,sInputData&id){id.Analog[0]=Mouse[0];id.Analog[1]=Mouse[1];}void Sample3DListener(sVector p,sVector v,sVector u,sVector f){listeners.push_back({p,v,u,f});}}systemObject;System*sSystem=&systemObject;'''+support[b:]
support=support.replace('int Life,Ammo[4];','int Life=0,Armor=0,Alpha=0,Beta=0,Ammo[4]{},Weapon[8]{},CurrentWeapon=0,NextWeapon=0,FireKey=0,UseKey=0;float CoolTimer=0;void Hit(int h){int d=h>Armor?h-Armor+Armor/4:h/4;Life=sMax(Life-d,0);}')
support=support.replace('struct KKriegerGame {','''struct Audio{int Slot;float Volume;sVector Position;};static std::vector<Audio>audio;
struct KKriegerShot{void Exit(){}};
struct KKriegerGame {int TickCount=0,WasInOptions=0,OptionsChanged=0,LastTime=0,FlyMode=0,SplashEnable=0,SplashHits=0;float MouseTurnSpeed=0.001f,MouseLookSpeed=0.002f,WeaponTimer=0.25f,WeaponCool[8]{};int WeaponFlags[8]{};KOp*WeaponShot[8]{};sVector SplashPos;KEnvironment*Environment;Array<KKriegerShot>Shots;
void OnTick(KEnvironment*,int);void OnOptionsChanged(){OptionsChanged++;}int SplashCalc(const sVector&,KKriegerCellAdd*){return 0;}void FireShot(KEnvironment*,int,KKriegerMonster*,const sVector*){}void MonsterMagnetAI(){}void MonsterAI(KKriegerMonster*,KEnvironment*,bool,bool){}bool ShotAI(KKriegerShot*){return false;}void Physic(float);void Zones(KEnvironment*){for(int i=0;i<DCellUsed;i++)DCellPtr[i]->Enter=DCellPtr[i]->Leave=DCellPtr[i]->Collided=0;}
''')
support=support.replace('void PlayerSound(int,float=1.f){SoundCount++;}', 'void PlayerSound(int slot,float volume=1.f){SoundCount++;audio.push_back({slot,volume,PlayerPos});}')
support=support.replace('void PrepareDynamic(sMatrix&);','void PrepareDynamic(sMatrix&);void ApplyMatrix(sMatrix&);')
support=support.replace('void InitEuler(float,float,float);','void Lin4(sMatrix&,sMatrix&,float);void InitEuler(float,float,float);')
support=support.replace('struct sVector {','struct sVector {void Lin3(const sVector&a,const sVector&b,float t){x=a.x+(b.x-a.x)*t;y=a.y+(b.y-a.y)*t;z=a.z+(b.z-a.z)*t;}')
support+='\nconstexpr float sPIF=3.14159265358979323846f;template<class T>T sRange(T v,T hi,T lo){return v>=hi?hi:v<=lo?lo:v;}\n'
support=support.replace('struct sVector {','struct sVector {void Rotate3(const sMatrix&,const sVector&);void RotatePl(const sMatrix&,const sVector&);void Scale4(float f){x*=f;y*=f;z*=f;w*=f;}')
support+='\nvoid sVector::Rotate3(const sMatrix&m,const sVector&v){Scale3(m.i,v.x);AddScale3(m.j,v.y);AddScale3(m.k,v.z);}void sVector::RotatePl(const sMatrix&m,const sVector&v){Rotate3(m,v);w=v.w-Dot3(m.l); }\n'
driver=restart.driver[:restart.driver.index('int main()')]+r'''
int main(){word(24);word(6);for(int c=0;c<24;c++){
 KKriegerGame game;KEnvironment env{&game};KKriegerCellAdd add;KKriegerCell floor{};sMatrix mat;mat.Init();sVector scale{20,20,20,0};add.Init(mat,scale,0);add.Id=0;*game.CellAdd.Add()=&add;*game.CellList.Add()=&add;sMatrix floorMat;floorMat.Init();floorMat.l.y=-1.5f;sVector floorScale{20,2,20,0};floor.Init(floorMat,floorScale,1);*game.CellSub.Add()=&floor;*game.CellList.Add()=&floor;game.CellConnect();game.CreateCollisionFaces(add);
 systemObject.Time=0;systemObject.Mouse[0]=systemObject.Mouse[1]=0;game.PlayerStartPos={1,2,3,1};game.SetPlayer({0,0,0,1},0,0);game.Player.Life=c%9;game.Player.Ammo[0]=5;game.GravityPlayer=0.01f;game.DampPlayerGround=0.05f;game.DampPlayerAir=0.025f;game.StairHeight=0.55f;game.DampPlayerCam=0.25f;game.CamOffset=0.85f;game.WeaponCool[0]=0.125f;game.DCellUsed=1;game.DCellPtr[0]=&game.Player.HitCell;game.JumpTimer=0;game.SpeedUp=0;game.OnGround=0;game.OffGroundTime=c%2?0.5f:0.f;game.StepFoot=0;game.StepTimer=0;game.AccelSide=0;game.AccelForw=0;game.PlayerMat.Init();word(game.Player.Life);
 for(int k=0;k<6;k++){
  int slices[]={0,1,3,2,-1,1},times[]={0,10,30,40,1050,1060};game.Switches[1]=(c%3==0 && k<2)?(k==0?9:1):0;game.Switches[16]=1;game.Switches[13]=c%10;game.Switches[12]=c%2;game.PlayerCell=c%7==0&&k==3?nullptr:&add;game.Player.FireKey=k%2;game.Player.UseKey=1;game.AccelForw=k%3==0?0.01f:0;game.AccelSide=k%3==1?0.005f:0;systemObject.Time=times[k];systemObject.Mouse[0]=c+k*7;systemObject.Mouse[1]=-c+k*5;
  word(slices[k]);word(times[k]);word(systemObject.Mouse[0]);word(systemObject.Mouse[1]);word(game.Switches[1]);word(game.Switches[13]);word(game.Switches[12]);word(game.PlayerCell!=nullptr);word(game.Player.FireKey);real(game.AccelForw);real(game.AccelSide);
  audio.clear();listeners.clear();game.OnTick(&env,slices[k]);word(game.TickCount);word(game.WasInOptions);word(game.OptionsChanged);word(game.LastTime);word(game.LastMouseX);word(game.LastMouseY);dump(game);for(auto v:{game.PlayerMat.i,game.PlayerMat.j,game.PlayerMat.k,game.PlayerMat.l})vector(v);word(game.Player.Life);word(game.Player.Ammo[0]);word(game.Player.FireKey);word(game.Player.UseKey);real(game.Player.CoolTimer);real(game.WeaponTimer);real(game.OffGroundTime);word(game.StepFoot);real(game.StepTimer);word(game.PlayerCell!=nullptr);word(listeners.size());for(auto v:listeners){vector(v.Pos);vector(v.Speed);vector(v.Up);vector(v.Forward);}word(audio.size());for(auto a:audio){word(a.Slot);real(a.Volume);vector(a.Position);}
 }
 add.DeleteFaces();
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'tick.cpp';exe=pathlib.Path(tmp)/'tick';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/tick-player.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 144 whole original player ticks')
