#!/usr/bin/env python3
"""Original monster damage, activation and byte-sized kill-switch updates."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
function=oracle.function('void KKriegerMonster::Hit(')
support=r'''
#include <cstdio>
using sInt=int;
#define KMS_INRANGE 3
#define KMS_ACTIVE 4
struct KKriegerGame {unsigned char Switches[256]{};};
struct KKriegerMonster {int Life,Armor,State,KillSwitch;void Hit(int,KKriegerGame*);};
void word(unsigned n){fwrite(&n,4,1,stdout);}
'''
driver=r'''
int main(){word(256);word(4);for(int c=0;c<256;c++){
 KKriegerGame game;KKriegerMonster mon{c%101-3,(c*7)%80,c%6,c%11==0?0:c%7==0?-1:(c*13)%256};int watch=mon.KillSwitch>0?mon.KillSwitch:0;game.Switches[watch]=250+c%6;
 word(mon.Life);word(mon.Armor);word(mon.State);word(mon.KillSwitch);word(watch);word(game.Switches[watch]);
 for(int k=0;k<4;k++){int hits=(k*17+c)%201-5;word(hits);mon.Hit(hits,&game);word(mon.Life);word(mon.Armor);word(mon.State);word(game.Switches[watch]);}
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'damage.cpp';exe=pathlib.Path(tmp)/'damage';cpp.write_text(support+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/monster-damage.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 1024 original monster damage calls')
