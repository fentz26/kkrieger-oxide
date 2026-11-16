#!/usr/bin/env python3
"""Extract the original Print expansion loop with original inventory bindings."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/geneffect.cpp').read_text(encoding='cp1252')
function=oracle.function('void __stdcall Exec_Effect_Print(')
start=function.index("      while(*text!=0 && *text!='\\n')")
end=function.index('      *p++ =0 ;',start)+len('      *p++ =0 ;')
formatter=function[start:end]
support=r'''
#include <cstdio>
#include <cstring>
#include <vector>
#include <string>
#include <initializer_list>
using sInt=int;using sChar=char;using sBool=bool;
#define sPLAYER 1
struct Player{int Armor,Life,Ammo[4],CurrentWeapon;};struct Game{struct Player Player;unsigned char Switches[256];};struct KEnvironment{struct Game*Game;};
float GlobalFps;
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
struct Line{std::string text;int center,newlines;};
'''
driver=r'''
int main(){const char*patterns[]={
"%3l; %2a; %4s259; %2w5; %3W; %5f;%|X\n\nEND",
"%s3zero|one|two\\|bar|three;", "%i3=2yes|no;/%i3!=2yes|no;/%i3!2yes|no;",
"%i3=0a|b;/%i3=1a|b;/%i3=3a|b;", "%0a;X/%9l;/%1l;", "%3l", "%3l0", "%1x0;",
"%s259a|b|c|d;", "%i259=2a\\;b|c;", "%|one%|two", "\n\nA\nB\n\n",
"%2W;%2w0;%2w1;%2w2;%2w3;", "%s3a|b|c|d", "%i3=2a|b", "%3l;/%3a;/%6f;"};
word(512);for(int c=0;c<512;c++){struct Game game{};KEnvironment env{&game};auto*kenv=&env;
game.Player.Life=(c*71)%2000-100;game.Player.Armor=(c*13)%200-20;for(int i=0;i<4;i++)game.Player.Ammo[i]=(c*17+i*31)%500-10;game.Player.CurrentWeapon=(c/16)%8;GlobalFps=17.125f+(c%128)*.25f;
for(int i=0;i<256;i++)game.Switches[i]=(i+c/16)%4;
for(int n:{game.Player.Life,game.Player.Armor,0,0,game.Player.Ammo[0],game.Player.Ammo[1],game.Player.Ammo[2],game.Player.Ammo[3],game.Player.CurrentWeapon})word(n);real(GlobalFps);for(auto v:game.Switches)word(v);
char input[2048]{};strcpy(input,patterns[c%16]);word(strlen(input));for(auto*p=input;*p;p++)word((unsigned char)*p);
char*text=input;std::vector<Line>lines;
while(*text){char buffer[512]{};char*p=buffer;int centerchar=-1,i;bool inf;
'''
ending=r'''
int newlines=0;while(*text=='\n'){newlines++;text++;}lines.push_back({buffer,centerchar,newlines});}
word(lines.size());for(auto&line:lines){word(line.text.size());for(unsigned char b:line.text)word(b);word(line.center);word(line.newlines);}}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp=pathlib.Path(tmp)/'format.cpp';exe=pathlib.Path(tmp)/'format'
    cpp.write_text(support+driver+formatter+ending)
    subprocess.run(['clang++','-std=c++17','-O2',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/print-format.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 512 original Print expansion calls')
