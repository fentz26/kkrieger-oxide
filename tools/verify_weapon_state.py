#!/usr/bin/env python3
"""Original weapon animation, footsteps, switch decisions and firing prefix."""
import pathlib,subprocess,tempfile
root=pathlib.Path(__file__).resolve().parents[1]
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
a=source.index('  // weapon animation');b=source.index('  // splash damage',a)
animate='void Game::Animate(int slices){'+source[a:b]+'}\n'
a=source.index('  // fire?');b=source.index('  // the timesliced-loop',a)
fire='void Game::Fire(int slices){void*kenv=nullptr;'+source[a:b]+'}\n'
a=source.index('  if(Player.NextWeapon!=Player.CurrentWeapon && WeaponTimer>=1.0f)');b=source.index('  if(WeaponEvent.Op)',a)
switch='void Game::Switch(){'+source[a:b]+'}\n'
support=r'''
#include <cstdio>
#include <initializer_list>
#define KWF_CONTINUOUS 1
struct Op{int Id;};struct Event{Op*OpPtr=nullptr;int Resets=0;void Exit(){}void Init(){OpPtr=nullptr;Resets++;}};
struct Sound{int Slot;float Volume;};
struct Game{
 struct PlayerType{int NextWeapon,CurrentWeapon,FireKey;float CoolTimer;int Ammo[4];}Player;
 float WeaponTimer,AccelForw,AccelSide,StepTimer;int StepFoot,OnGround;
 float WeaponCool[8];int WeaponFlags[8];Op*WeaponShot[8];Op*WeaponOptics[8];Event WeaponEvent;
 Sound sounds[4];int SoundCount,ShotCount,ShotWeapon,ShotOperator;
 void PlayerSound(int slot,float volume){sounds[SoundCount++]={slot,volume};}
 void FireShot(void*,int weapon,int,int){ShotCount++;ShotWeapon=weapon;ShotOperator=WeaponShot[weapon]->Id;}
 void Animate(int);void Fire(int);void Switch();
};
void word(unsigned x){fwrite(&x,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
'''
# Rename only the recorder's pointer field to avoid the Op type/member collision.
switch=switch.replace('WeaponEvent.Op =','WeaponEvent.OpPtr =')
driver=r'''
int main(){word(64);word(16);for(int c=0;c<64;c++){
 Game g{};Op targets[16];for(int i=0;i<16;i++)targets[i].Id=i+1;
 for(int i=0;i<8;i++){g.WeaponCool[i]=0.05f+i*0.01f;g.WeaponFlags[i]=((i+c)%2)|((c%3)<<3);g.WeaponShot[i]=(i+c)%3?&targets[i]:nullptr;g.WeaponOptics[i]=(i+c)%4?&targets[8+i]:nullptr;real(g.WeaponCool[i]);word(g.WeaponFlags[i]);word(g.WeaponShot[i]?g.WeaponShot[i]->Id:~0u);word(g.WeaponOptics[i]?g.WeaponOptics[i]->Id:~0u);}
 g.Player.CurrentWeapon=c%8;g.Player.NextWeapon=(c/8)%8;g.Player.CoolTimer=c%3==0?0.05f:-0.01f;g.Player.FireKey=1;
 float timers[5]={0.24f,0.25f,0.74f,0.76f,1.2f};g.WeaponTimer=timers[c%5];g.StepFoot=c%2;g.StepTimer=0.49f;
 for(int i=0;i<4;i++)g.Player.Ammo[i]=(c+i)%3-1;
 word(g.Player.CurrentWeapon);word(g.Player.NextWeapon);real(g.Player.CoolTimer);word(g.Player.FireKey);real(g.WeaponTimer);word(g.StepFoot);real(g.StepTimer);for(int n:g.Player.Ammo)word(n);
 for(int k=0;k<16;k++){
  int slices[5]={0,1,5,40,100};int s=slices[(k+c)%5];g.AccelForw=(k%5-2)*0.005f;g.AccelSide=(k%3-1)*0.005f;g.OnGround=(c+k)%2;
  int key=k%7==0?2:k%5==0?1:0;if(key)g.Player.FireKey=key==1;
  word(s);real(g.AccelForw);real(g.AccelSide);word(g.OnGround);word(key);g.SoundCount=g.ShotCount=0;
  g.Switch();g.Animate(s);g.Fire(s);
  word(g.Player.CurrentWeapon);word(g.Player.NextWeapon);real(g.Player.CoolTimer);word(g.Player.FireKey);real(g.WeaponTimer);word(g.StepFoot);real(g.StepTimer);for(int n:g.Player.Ammo)word(n);
  word(g.WeaponEvent.Resets);word(g.WeaponEvent.OpPtr?g.WeaponEvent.OpPtr->Id:~0u);word(g.SoundCount);for(int i=0;i<g.SoundCount;i++){word(g.sounds[i].Slot);real(g.sounds[i].Volume);}
  word(g.ShotCount);if(g.ShotCount){word(g.ShotWeapon);word(g.ShotOperator);}
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'weapon.cpp';exe=pathlib.Path(tmp)/'weapon';cpp.write_text(support+switch+animate+fire+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/weapon-state.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 1024 original weapon frames')
