#!/usr/bin/env python3
"""Compile unchanged original Zones with memory-only cell/event stubs."""
import pathlib,re,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
oracle.source=(base/'kkriegergame.cpp').read_text(encoding='cp1252')
functions='\n'.join(oracle.function(s) for s in ['void KKriegerGame::Zones(','void KKriegerCell::DoRespawn('])
defines='\n'.join(line for line in (base/'kdoc.hpp').read_text(encoding='cp1252').splitlines() if re.match(r'#define KKE[EC]_',line))
# The obsolete logic aliases are inside #if 0 in the source header.
defines='\n'.join(line for line in defines.splitlines() if not any(n in line for n in ['KKEC_ANDLOGIC','KKEC_ORLOGIC','KKEC_NOTLOGIC']))
support=r'''
#include <cstdio>
#include <vector>
using sInt=int;
#define KCM_ZONE 2
#define KPM_PLAYER 2
#define KPM_WEAPONS 0xff00
#define KGS_RESPAWN 32
#define sVERIFY(x) ((void)0)
template<class T>T sMin(T a,T b){return a<b?a:b;}template<class T>T sMax(T a,T b){return a>b?a:b;}
template<class T>struct Array{std::vector<T>v;int Count=0;T&operator[](int i){return v[i];}void add(T x){v.push_back(x);Count++;}};
struct Instance{int Deleted=0;void DeleteChain(){Deleted++;}};struct Event{Instance*FirstInstance;};
struct KKriegerCell{int Mode=2,Respawn=0;unsigned Enter=0,Leave=0,Collided=0;struct{int Code,Condition,Value,Output;}Logic;Event*Event[2]{};void DoRespawn();};
using KKriegerCellDynamic=KKriegerCell;
struct KEnvironment{};
struct KKriegerGame{unsigned char Switches[256]{};Array<KKriegerCell*>CellZone;int DCellUsed=2;KKriegerCellDynamic*DCellPtr[2];float SpeedUp=0;struct{int UseKey,NextWeapon;int Life,Armor,Alpha,Beta,Ammo[4],Weapon[8];int LifeMax,ArmorMax,AlphaMax,BetaMax,AmmoMax[4],WeaponMax[8];}Player{};void Zones(KEnvironment*);};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
void cellout(KKriegerCell&z){word(z.Respawn);word(z.Enter);word(z.Leave);word(z.Collided);}
'''
driver=r'''
int main(){word(832);word(4);for(int c=0;c<832;c++){
 KKriegerGame game;KKriegerCell zones[3],dynamic[2];Instance instances[3];Event events[3];
 for(int i=0;i<256;i++)game.Switches[i]=(c+i)%5;for(auto b:game.Switches)word(b);
 game.Player.UseKey=c%2;game.Player.NextWeapon=3;game.SpeedUp=0.25f;
 word(game.Player.UseKey);word(game.Player.NextWeapon);real(game.SpeedUp);
 for(int i=0;i<16;i++){(&game.Player.Life)[i]=(c+i)%17-3;(&game.Player.LifeMax)[i]=i<8?100+i*10:1;word((&game.Player.Life)[i]);word((&game.Player.LifeMax)[i]);}
 int timers[]={0,1,10,11,20,10000,0x40000000,0x7fffffff};word(3);
 for(int i=0;i<3;i++){
  auto&z=zones[i];z.Logic.Code=i==0?((c%13)<<4)|((c/13)%16):i==1?0xa3:0xb8;
  z.Logic.Condition=i==0?(c%4):i==1?6:7;z.Logic.Output=i==0?(c/208)*8+(c%8):i==1?7:12;
  z.Logic.Value=(c*7+i*13)%101-20;z.Respawn=timers[(c+i)%8];events[i].FirstInstance=&instances[i];z.Event[1]=&events[i];
  word(z.Logic.Code);word(z.Logic.Condition);word(z.Logic.Output);word(z.Logic.Value);word(z.Respawn);game.CellZone.add(&z);
 }
 for(int i=0;i<2;i++)game.DCellPtr[i]=&dynamic[i];
 for(int tick=0;tick<4;tick++){
  for(int i=0;i<3;i++){auto&z=zones[i];unsigned masks[]={0,2,0x100,0xff02};z.Enter=masks[(c+i+tick)%4];z.Leave=masks[(c/4+i+tick)%4];z.Collided=masks[(c/16+i+tick)%4];word(z.Enter);word(z.Leave);word(z.Collided);}
  for(auto&z:dynamic){z.Enter=2;z.Leave=0xff00;z.Collided=0xff02;}
  game.Zones(nullptr);
  for(auto b:game.Switches)word(b);word(game.Player.NextWeapon);real(game.SpeedUp);for(int i=0;i<16;i++)word((&game.Player.Life)[i]);
  for(int i=0;i<3;i++){cellout(zones[i]);word(instances[i].Deleted);word(events[i].FirstInstance!=nullptr);}
  for(auto&z:dynamic)cellout(z);
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'zones.cpp';exe=pathlib.Path(tmp)/'zones';cpp.write_text(support+defines+'\n'+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/zones.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 3328 original zone phases')
