#!/usr/bin/env python3
"""Original player-build Flush field reset and ownership callbacks."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252');function=oracle.function('void KKriegerGame::Flush(')
support=r'''
#include <cstdio>
#include <vector>
using sInt=int;
#define sPLAYER 1
int listExits,facesDeleted,cellsDeleted,eventExits,shotExits;
struct List{void Exit(){listExits++;}};
struct Event{float Modulation;void Exit(){eventExits++;}};
struct Cell{List Adds,Subs,Zones;struct Event*Event[2]={nullptr,nullptr};void DeleteFaces(){facesDeleted++;}~Cell(){cellsDeleted++;}};
struct Shot{void Exit(){shotExits++;}};
template<class T>struct Array{std::vector<T>v;int Count=0;T&operator[](int i){return v[i];}};
struct Player{int CurrentWeapon,NextWeapon,Life,FireKey,UseKey;float CoolTimer;};
struct KKriegerGame{Array<Cell*>CellAdd,CellSub,CellZone;Array<Shot>Shots;void*WeaponShot[8],*WeaponOptics[8],*WeaponExplode[2][8];int PartUsed,ConsUsed,StepFoot;float WeaponTimer,StepTimer,OffGroundTime,AccelForw,SpeedUp;struct Player Player;Event WeaponEvent;void Flush();};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
'''
driver=r'''
int main(){word(64);for(int c=0;c<64;c++){
 listExits=facesDeleted=cellsDeleted=eventExits=shotExits=0;KKriegerGame g{};
 for(int i=0;i<c%5;i++)g.CellAdd.v.push_back(new Cell);g.CellAdd.Count=g.CellAdd.v.size();
 for(int i=0;i<c%3;i++)g.CellSub.v.push_back(new Cell);g.CellSub.Count=g.CellSub.v.size();
 for(int i=0;i<c%4;i++){auto cell=new Cell;cell->Event[0]=new Event;if(i%2)cell->Event[1]=new Event;g.CellZone.v.push_back(cell);}g.CellZone.Count=g.CellZone.v.size();g.Shots.v.resize(c%7);g.Shots.Count=g.Shots.v.size();
 for(int i=0;i<8;i++){g.WeaponShot[i]=(void*)1;g.WeaponOptics[i]=(void*)1;g.WeaponExplode[0][i]=(void*)1;g.WeaponExplode[1][i]=(void*)1;}
 g.PartUsed=g.ConsUsed=g.StepFoot=c+9;g.WeaponTimer=g.StepTimer=g.OffGroundTime=c+0.5f;g.Player.CurrentWeapon=7;g.Player.NextWeapon=6;g.Player.Life=20+c;g.Player.FireKey=c%2;g.Player.UseKey=c%2;g.Player.CoolTimer=c*0.125f;g.WeaponEvent.Modulation=c*0.25f;g.AccelForw=c*0.0625f;g.SpeedUp=c*-0.125f;
 g.Flush();for(int n:{g.CellAdd.Count,g.CellSub.Count,g.CellZone.Count,g.Shots.Count,g.PartUsed,g.ConsUsed})word(n);for(float f:{g.WeaponTimer,g.StepTimer,g.OffGroundTime})real(f);word(g.StepFoot);word(g.Player.CurrentWeapon);word(g.Player.NextWeapon);for(int i=0;i<8;i++){word(g.WeaponShot[i]!=nullptr);word(g.WeaponOptics[i]!=nullptr);word(g.WeaponExplode[0][i]!=nullptr);word(g.WeaponExplode[1][i]!=nullptr);}
 for(int n:{g.Player.Life,g.Player.FireKey,g.Player.UseKey})word(n);for(float f:{g.Player.CoolTimer,g.WeaponEvent.Modulation,g.AccelForw,g.SpeedUp})real(f);for(int n:{listExits,facesDeleted,cellsDeleted,eventExits,shotExits})word(n);
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'flush.cpp';exe=pathlib.Path(tmp)/'flush';cpp.write_text(support+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/flush.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 64 original game Flush calls')
