#!/usr/bin/env python3
"""Original player-build OnKey with memory-only respawn placement and sound IO."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_player_actions as actions
root=oracle.root;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
functions='\n'.join(oracle.function(s) for s in ['sBool KKriegerGame::OnKey(','void KKriegerGame::RespawnAt(','void KKriegerPlayer::Hit(','void KKriegerPlayer::Sound(','void KKriegerGame::PlayerSound('])
support=actions.support
support=support.replace('using sInt=int;', 'using sBool=bool;using sU32=unsigned;using sS8=signed char;constexpr bool sTRUE=true,sFALSE=false;using sInt=int;')
support='#include <cstring>\n#define sKEYQ_BREAK 0x80000000u\n#define sKEY_SHIFTL 0x10021\n#define sKEY_CTRLR 0x10025\n#define sKEY_MOUSEL 0x10028\n'+support
support=support.replace('int Life,Armor;', 'int UseKey=0,FireKey=0,NextWeapon=0;int Life,Armor,Alpha,Beta,Ammo[4],Weapon[8];int LifeMax,ArmorMax,AlphaMax,BetaMax,AmmoMax[4],WeaponMax[8];')
a=support.index('struct Game{');b=support.index('void word(',a)
support=support[:a]+r'''
struct Checkpoint{int Valid=0;Vec Pos;float Dir=0;};
struct KKriegerGame{int OnGround=1,JumpTimer=0,FlyMode=0;unsigned LastKey=0;float SpeedUp=-0.125f,JumpForce=0.25f,AccelForw=0,AccelSide=0,AccelForwFactor=0.01f,AccelBackFactor=0.005f,AccelSideFactor=0.0075f;Vec PlayerPos;KKriegerPlayer Player;Checkpoint Respawn[16];int PlacementCount=0,LastMouseX=0,LastMouseY=0,MouseX=0,MouseY=0;
 bool OnKey(unsigned);void RespawnAt(int);void PlayerSound(int,float=1.f);void SetPlayer(const Vec&,float,float){PlacementCount++;LastMouseX=MouseX;LastMouseY=MouseY;}};
void sCopyMem(void*d,const void*s,int n){memcpy(d,s,n);}
'''+support[b:]
driver=r'''
int main(){word(64);word(48);unsigned keys[]={ 'm','n','M','2','w','m','a'|sKEYQ_BREAK,'n',' ', 'K',sKEY_SHIFTL,sKEY_MOUSEL,sKEY_MOUSEL|sKEYQ_BREAK,sKEY_CTRLR,sKEY_CTRLR|sKEYQ_BREAK,'y','Y','1','2','3','4','5','6','7','8','9','m','1','M','3','s','S'|sKEYQ_BREAK,'D','d'|sKEYQ_BREAK,'A','a'|sKEYQ_BREAK,'W'|sKEYQ_BREAK,'r','I','z',sKEY_SHIFTL|sKEYQ_BREAK, 'N','m','9','M','5','k',0};
 for(int c=0;c<64;c++){
  KKriegerGame game;game.OnKey('w');game.AccelForw=0;
  for(int i=0;i<16;i++){(&game.Player.Life)[i]=(c+i)%11;(&game.Player.LifeMax)[i]=i<8?100+i*10:1;word((&game.Player.Life)[i]);word((&game.Player.LifeMax)[i]);game.Respawn[i].Valid=(c+i)%3!=0;word(game.Respawn[i].Valid);}
  for(int k=0;k<48;k++){
   unsigned key=keys[k]|((c%2)?0x40000000u:0);game.MouseX=c*37+k;game.MouseY=-c*11+k*3;game.OnGround=(c+k)%3!=0;game.JumpTimer=(c+k)%4;game.SpeedUp=-0.125f;
   word(key);word(game.MouseX);word(game.MouseY);word(game.OnGround);word(game.JumpTimer);real(game.SpeedUp);
   soundCount=0;bool handled=game.OnKey(key);
   word(handled);word(game.LastKey);word(game.FlyMode);real(game.AccelForw);real(game.AccelSide);real(game.SpeedUp);word(game.JumpTimer);word(game.Player.UseKey);word(game.Player.FireKey);word(game.Player.NextWeapon);word(game.PlacementCount);word(game.LastMouseX);word(game.LastMouseY);
   for(int i=0;i<16;i++)word((&game.Player.Life)[i]);word(soundCount);for(int i=0;i<soundCount;i++){word(sounds[i].Slot);real(sounds[i].Volume);real(sounds[i].Near);real(sounds[i].Far);}
  }
 }
}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'input.cpp';exe=pathlib.Path(tmp)/'input';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/input-keys.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 3072 original key events')
