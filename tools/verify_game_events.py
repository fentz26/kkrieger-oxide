#!/usr/bin/env python3
"""Unchanged game AddEvents with native scheduler and memory-only objects."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_events as events
root=oracle.root;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252');function=oracle.function('void KKriegerGame::AddEvents(')
support=events.support.replace('using sInt=int;', 'constexpr float sPI2F=6.2831853071795864769f;using sInt=int;')
support=support.replace('void InitS(float f)', 'void Init(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}void InitS(float f)')
support=support.replace('void Stop();', 'void Init(){int id=Id;*this=KEvent{};Id=id;}void Exit(){if(FirstInstance){FirstInstance->DeleteChain();FirstInstance=nullptr;}}void Stop();')
support+='''
struct KKriegerShot{KEvent*Event;int Mode,Matrix;};
struct KKriegerCell{KEvent*Event[2];int Respawn;};
struct KKriegerGame{Array<KKriegerShot>Shots;Array<KKriegerCell*>CellZone;struct{int CurrentWeapon,NextWeapon;float CoolTimer;}Player;KEvent WeaponEvent;KOp*WeaponOptics[8];float WeaponTimer,LastCamY,CamOffset,PlayerLook,PlayerDir,WeaponCool[8];sVector PlayerPos;void AddEvents(KEnvironment*);};
'''
driver=r'''
int main(){word(96);word(4);for(int c=0;c<96;c++){
 KEnvironment env;KOp ops[3];KInstanceMem mem;KEvent events[12];KKriegerCell zones[3];KKriegerGame game;
 for(int i=0;i<12;i++){events[i].Id=i;events[i].Op=&ops[(c+i)%3];events[i].FirstInstance=&mem;}
 word(6);for(int i=0;i<6;i++){events[i].Select=(c+i)%5==0;*game.Shots.Add()={&events[i],i%3,100+i};word((c+i)%3);word(events[i].Select);word(i%3);word(100+i);}
 for(int i=0;i<3;i++){zones[i].Event[0]=(c+i)%4==0?nullptr:&events[6+2*i];zones[i].Event[1]=(c+i)%5==0?nullptr:&events[7+2*i];*game.CellZone.Add()=&zones[i];for(auto e:zones[i].Event){word(e!=nullptr);if(e)word((c+e->Id)%3);}}
 game.WeaponEvent.Id=12;game.WeaponEvent.Op=c%3==0?nullptr:&ops[c%3];game.WeaponEvent.FirstInstance=&mem;word(game.WeaponEvent.Op!=nullptr);if(game.WeaponEvent.Op)word(c%3);
 game.Player.CurrentWeapon=c%8;game.Player.NextWeapon=(c+1)%8;word(game.Player.CurrentWeapon);word(game.Player.NextWeapon);
 for(int i=0;i<8;i++){game.WeaponOptics[i]=(c+i)%4==0?nullptr:&ops[i%3];game.WeaponCool[i]=i%3==0?0.01f:i%3==1?0.f:0.125f*(i+1);word(game.WeaponOptics[i]!=nullptr);if(game.WeaponOptics[i])word(i%3);real(game.WeaponCool[i]);}
 for(int tick=0;tick<4;tick++){
  env.BeatTime=0;deletes=0;
  game.WeaponTimer=tick*0.5f;game.Player.CoolTimer=(c%7)*0.1f;game.PlayerPos={c*0.25f,tick*0.5f,-2.f,1.f};game.LastCamY=-c*0.125f;game.CamOffset=0.85f;game.PlayerLook=(c%7-3)*0.125f;game.PlayerDir=(c%9-4)*0.25f;
  real(game.WeaponTimer);real(game.Player.CoolTimer);vec(game.PlayerPos);for(float f:{game.LastCamY,game.CamOffset,game.PlayerLook,game.PlayerDir})real(f);
  for(int i=0;i<3;i++){zones[i].Respawn=(c+i+tick)%3-1;word(zones[i].Respawn);}
  game.AddEvents(&env);
  word(deletes);word(game.Player.CurrentWeapon);word(game.WeaponTimer==0);real(game.WeaponTimer);word(game.Shots.Count);for(int i=0;i<game.Shots.Count;i++)word(game.Shots[i].Event->Id);
  for(auto&op:ops){int count=0;for(auto e=op.FirstEvent;e;e=e->NextOp)count++;word(count);for(auto e=op.FirstEvent;e;e=e->NextOp)word(e->Id);}
  for(int i=0;i<6;i++)word(events[i].Matrix);
  word(game.WeaponEvent.Op!=nullptr);if(game.WeaponEvent.Op)word(game.WeaponEvent.Op-&ops[0]);vec(game.WeaponEvent.Translate);vec(game.WeaponEvent.Rotate);real(game.WeaponEvent.Velocity);real(game.WeaponEvent.Modulation);word(game.WeaponEvent.FirstInstance!=nullptr);
  env.ExitFrame();
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'game_events.cpp';exe=pathlib.Path(tmp)/'game_events';cpp.write_text(support+events.functions+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/game-event-registration.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 384 original game event registration phases')
