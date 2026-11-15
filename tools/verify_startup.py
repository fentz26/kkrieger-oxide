#!/usr/bin/env python3
"""Original player initialization, Panic defaults, switch init and root decisions."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger';source=(base/'kkriegergame.cpp').read_text(encoding='cp1252');oracle.source=source
functions='\n'.join(oracle.function(s) for s in ['void KKriegerPlayer::Init(','void KKriegerGame::Panic(','sInt KKriegerGame::GetNewRoot('])
a=source.index('  sSetMem(Switches,0,KKRIEGER_SWITCHES);',source.index('void KKriegerGame::Init()'));b=source.index('\n}',a);functions+='\nvoid KKriegerGame::InitSwitches(){\n'+source[a:b]+'\n}\n'
support=r'''
#include <cstdio>
#include <cstring>
using sInt=int;using sF32=float;
#define sPLAYER 1
#define sLINK_KKRIEGER 1
#define sFALSE false
#define KKRIEGER_SWITCHES 256
#define KGS_GAME 1
#define KGS_GAME_RUN 0
#define KGS_GAME_INGAME 1
#define KGS_GAME_RESTART 2
#define KGS_GAME_QUIT 3
#define KGS_GAME_START 6
#define KGS_GAME_INTRO 8
#define KGS_ONE 0
#define KGS_RESOLUTION 8
#define KGS_TEXTURES 9
#define KGS_SHADOWS 10
#define KGS_GLARE 11
#define KGS_MOUSESPEED 13
#define KGS_BRIGHTNESS 14
#define KGS_SPECULAR 15
void sSetMem(void*p,int v,int n){memset(p,v,n);}
struct sVector{float x,y,z,w;void Init(float a=0,float b=0,float c=0,float d=1){x=a;y=b;z=c;w=d;}};
struct sInputData{int Analog[2];};struct System{int Exits=0;void Exit(){Exits++;}void GetInput(int,sInputData&id){id.Analog[0]=3;id.Analog[1]=7;}}systemObject;System*sSystem=&systemObject;
struct KKriegerPlayer{int Life,Armor,Alpha,Beta,Ammo[4],Weapon[8],LifeMax,ArmorMax,AlphaMax,BetaMax,AmmoMax[4],WeaponMax[8],CurrentWeapon,NextWeapon,FireKey,UseKey;float CoolTimer;void Init();};
struct KKriegerGame{int OnGround,ResetByOp,JumpTimer,CollisionForMonsterMode,FlyMode,SplashEnable,CamZoomPos,LastMouseX,LastMouseY,WasInOptions,Options=0,Restarts=0;float SpeedForw,AccelForw,SpeedSide,AccelSide,SpeedUp,Gravity,GravityPlayer,DampPlayerAir,DampPlayerCam,DampPlayerGround,DampPlayerHeight,StairHeight,EyeHeight,MouseTurnSpeed,MouseLookSpeed,AccelSideFactor,AccelForwFactor,AccelBackFactor,JumpForce,CamOffset;unsigned char Switches[256];void Panic();int GetNewRoot();void Restart(){Restarts++;}void OnOptionsChanged(){Options++;}void InitSwitches();};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
'''
driver=r'''
int main(){word(1);KKriegerPlayer player{};KKriegerGame game{};player.Init();game.Panic();game.InitSwitches();for(int i=0;i<16;i++)word((&player.Life)[i]);for(int i=0;i<16;i++)word((&player.LifeMax)[i]);for(int v:{player.CurrentWeapon,player.NextWeapon,player.FireKey,player.UseKey})word(v);real(player.CoolTimer);
for(float f:{game.GravityPlayer,game.DampPlayerGround,game.DampPlayerAir,game.DampPlayerHeight,game.StairHeight,game.EyeHeight,game.MouseTurnSpeed,game.MouseLookSpeed,game.AccelSideFactor,game.AccelForwFactor,game.AccelBackFactor,game.JumpForce,game.DampPlayerCam,game.CamOffset})real(f);word(game.ResetByOp);for(auto b:game.Switches)word(b);word(game.WasInOptions);word(game.Options);
word(256);for(int c=0;c<256;c++){for(int i=0;i<256;i++)game.Switches[i]=(c+i)%7;game.Switches[1]=c;for(auto b:game.Switches)word(b);game.Restarts=0;systemObject.Exits=0;word(game.GetNewRoot());word(game.Restarts);word(systemObject.Exits);for(auto b:game.Switches)word(b);}}
'''
support='#include <initializer_list>\n'+support
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'startup.cpp';exe=pathlib.Path(tmp)/'startup';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/startup.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, original startup defaults and 256 root selections')
