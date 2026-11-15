#!/usr/bin/env python3
"""Original parameter operator across repeated calls and explicit resets."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
function=oracle.function('void __stdcall Exec_KKrieger_Para(')
support=r'''
#include <cstdio>
#include <cstring>
#include <initializer_list>
using sInt=int;using sU32=unsigned;using sF32=float;
#define __stdcall
#define sCopyMem memcpy
struct Vec{float x,y,z,w;void Init(float a,float b,float c){x=a;y=b;z=c;w=1;}};
struct PlayerType{int Life,Armor,Alpha,Beta,Ammo[4],Weapon[8],LifeMax,ArmorMax,AlphaMax,BetaMax,AmmoMax[4];};
struct GameType {PlayerType Player{};bool ResetByOp=true;
 float GravityPlayer,DampPlayerGround,DampPlayerAir,DampPlayerHeight,StairHeight,EyeHeight,MouseTurnSpeed,MouseLookSpeed,AccelSideFactor,AccelForwFactor,AccelBackFactor,JumpForce;Vec PlayerStartPos;float DampPlayerCam;
 int WeaponHits[8];float WeaponCool[8];int WeaponFlags[8];float CamOffset;
};
struct KEnvironment{GameType*Game;};struct KOp{unsigned words[57];unsigned*GetAnimPtrU(int){return words;}float*GetAnimPtrF(int){return (float*)words;}void ExecInputs(KEnvironment*){}};
void word(unsigned x){fwrite(&x,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
'''
driver=r'''
int main(){word(128);word(3);for(int c=0;c<128;c++){GameType game{};KEnvironment env{&game};KOp op;
 for(int k=0;k<3;k++){
  if(k==2)game.ResetByOp=true;word(game.ResetByOp);
  for(int i=0;i<57;i++)op.words[i]=(c*97+k*43+i*11)^(i%3==0?0xf0000000u:0);
  for(int i=17;i<=31;i++){float f=(c+k+i-25)*0.03125f;memcpy(&op.words[i],&f,4);}
  for(int i=40;i<48;i++){float f=(c+k+i)*0.0625f;memcpy(&op.words[i],&f,4);}
  float damp=(c+k)*0.0078125f;memcpy(&op.words[56],&damp,4);for(auto v:op.words)word(v);
  Exec_KKrieger_Para(&op,&env);word(game.ResetByOp);
  for(int v:{game.Player.Life,game.Player.Armor,game.Player.Alpha,game.Player.Beta})word(v);for(int v:game.Player.Ammo)word(v);
  for(int v:{game.Player.LifeMax,game.Player.ArmorMax,game.Player.AlphaMax,game.Player.BetaMax})word(v);for(int v:game.Player.AmmoMax)word(v);for(int v:game.Player.Weapon)word(v);
  for(float f:{game.GravityPlayer,game.DampPlayerGround,game.DampPlayerAir,game.DampPlayerHeight,game.StairHeight,game.EyeHeight,game.MouseTurnSpeed,game.MouseLookSpeed,game.AccelSideFactor,game.AccelForwFactor,game.AccelBackFactor,game.JumpForce,game.PlayerStartPos.x,game.PlayerStartPos.y,game.PlayerStartPos.z,game.PlayerStartPos.w,game.DampPlayerCam})real(f);
  for(int v:game.WeaponHits)word(v);for(float f:game.WeaponCool)real(f);for(int v:game.WeaponFlags)word(v);real(game.CamOffset);
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'parameters.cpp';exe=pathlib.Path(tmp)/'parameters';cpp.write_text(support+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off','-fno-strict-aliasing',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/game-parameters.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 384 original parameter applications')

events=oracle.function('void __stdcall Exec_KKrieger_Events(')
event_support=support.replace('struct GameType {','struct KOp;\nstruct GameType {KOp*WeaponShot[8]{};KOp*WeaponOptics[8]{};KOp*WeaponExplode[2][8]{};')
event_support=event_support.replace('struct KOp{unsigned words[57];','struct KOp{unsigned Id;KOp*links[8];KOp*GetLink(int i){return links[i];}unsigned words[57];')
event_driver=r'''
int main(){word(64);word(6);for(int c=0;c<64;c++){
 GameType game{};KEnvironment env{&game};KOp op{},targets[8]{};for(int i=0;i<8;i++)targets[i].Id=i+1;
 unsigned kinds[6]={0,1,2,3,17,0xffffffffu};
 for(int k=0;k<6;k++){
  op.words[0]=kinds[k];word(kinds[k]);for(int i=0;i<8;i++){op.links[i]=(i+k+c)%3?&targets[(i+c)%8]:nullptr;word(op.links[i]?op.links[i]->Id:~0u);}
  Exec_KKrieger_Events(&op,&env);for(auto group:{game.WeaponShot,game.WeaponOptics,game.WeaponExplode[0],game.WeaponExplode[1]})for(int i=0;i<8;i++)word(group[i]?group[i]->Id:~0u);
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'events.cpp';exe=pathlib.Path(tmp)/'events';cpp.write_text(event_support+events+event_driver)
 subprocess.run(['clang++','-std=c++17','-O2',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/game-events.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 384 original weapon-event registrations')
