#!/usr/bin/env python3
"""Original jump-key action and player damage, preserving sound requests."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
hit=oracle.function('void KKriegerPlayer::Hit(')
start=oracle.source.index('    if(OnGround && JumpTimer==0)')
audio=oracle.function('void KKriegerPlayer::Sound(')+'\n'+oracle.function('void KKriegerGame::PlayerSound(').replace('KKriegerGame::','Game::')
jump='void Game::Jump(){'+oracle.source[start:oracle.source.index('    break;',start)]+'}\n'
support=r'''
#include <cstdio>
using sInt=int;using sF32=float;
#define sPLAYER 1
struct Vec{float x=0,y=0,z=0,w=0;void Init(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}};using sVector=Vec;
struct SoundRecord{int Slot;float Volume,Near,Far;};SoundRecord sounds[4];int soundCount;
struct System{int SamplePlay(int slot,float volume){int b=soundCount++;sounds[b]={slot,volume,0,0};return b;}void Sample3DParam(int,int b,const Vec&,const Vec&,float near,float far){sounds[b].Near=near;sounds[b].Far=far;}}systemObject;System*sSystem=&systemObject;
struct KKriegerPlayer{int Life,Armor;struct{Vec Pos;}Collider;void Sound(int,float=1.f);void Hit(int);};
struct Game{int OnGround,JumpTimer;float SpeedUp,JumpForce;Vec PlayerPos;void PlayerSound(int,float);void Jump();};
void word(unsigned v){fwrite(&v,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
'''
driver=r'''
int main(){word(256);word(4);for(int c=0;c<256;c++){
 KKriegerPlayer player{c%151,(c*7)%101};Game game;word(player.Life);word(player.Armor);
 for(int k=0;k<4;k++){
  int hits=(c*3+k*13)%201-5;game.OnGround=(c+k)%2;game.JumpTimer=(c+k)%3;game.SpeedUp=-0.125f;game.JumpForce=(c%11+1)*0.03125f;
  word(hits);word(game.OnGround);word(game.JumpTimer);real(game.SpeedUp);real(game.JumpForce);
  soundCount=0;game.Jump();player.Hit(hits);
  word(player.Life);word(player.Armor);word(game.JumpTimer);real(game.SpeedUp);word(soundCount);for(int i=0;i<soundCount;i++){word(sounds[i].Slot);real(sounds[i].Volume);real(sounds[i].Near);real(sounds[i].Far);}
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'actions.cpp';exe=pathlib.Path(tmp)/'actions';cpp.write_text(support+hit+audio+jump+driver)
 subprocess.run(['clang++','-std=c++17','-O2',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/player-actions.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 1024 original jump/damage pairs')
