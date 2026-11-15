#!/usr/bin/env python3
"""Exercise the original Delay handler across persistent, changing inputs."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle

root = oracle.root
oracle.source = (root / '.reference/fr_public/werkkzeug3_kkrieger/kdoc.cpp').read_text(encoding='cp1252')
function = oracle.function('void __stdcall Exec_Misc_Delay(')
support = r'''
#include <cstdio>
using sInt=int;using sF32=float;
#define __stdcall
struct KInstanceMem {bool Reset=false;};
struct DelayMem:KInstanceMem {int Phase=0;float Time=0;};
struct Vec {float x=0,y=0,z=0,w=0;void Init(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}};
struct Game {unsigned char *Switches;};struct KOp;
struct KEnvironment {Vec Var[4];Game*Game;int CurrentTime,LastTime;DelayMem mem;template<class T>T*GetInst(KOp*){return (T*)&mem;}};
struct KOp {void ExecInput(KEnvironment*,int){}};
void word(unsigned x){fwrite(&x,4,1,stdout);}void real(float x){fwrite(&x,4,1,stdout);}
'''
driver = r'''
int main(){word(256);word(80);for(int c=0;c<256;c++){
 KEnvironment e{};unsigned char sw[1]={};Game game{sw};e.Game=&game;KOp op;
 int flags=((c%4)<<6)|(((c/4)%4)<<4)|(c%5);float threshold=(c%3)*0.25f,hyst=(c%4)*0.125f;
 float t[3]={(c%5-1)*0.25f,((c/5)%5-1)*0.125f,((c/25)%5-1)*0.5f};
 word(flags);real(threshold);real(hyst);for(float f:t)real(f);
 for(int k=0;k<80;k++){
  e.mem.Reset=k==0;e.LastTime=k*123;e.CurrentTime=e.LastTime+(k%11==0?-100:k%13==0?2500:123);
  sw[0]=(k/9)%3;word(sw[0]);word(e.CurrentTime);word(e.LastTime);
  for(int v=0;v<4;v++){float f=((k/7+v+c)%7-2)*0.25f;e.Var[v].Init(f,f+0.25f,f+0.5f,f+0.75f);for(float x:{e.Var[v].x,e.Var[v].y,e.Var[v].z,e.Var[v].w})real(x);}
  Exec_Misc_Delay(&op,&e,flags,0,threshold,hyst,t[0],t[1],t[2]);
  word(e.mem.Phase);real(e.mem.Time);for(auto v:e.Var)for(float x:{v.x,v.y,v.z,v.w})real(x);
 }
}}
'''
# Initializer-list range loops need this standard header.
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'delay.cpp'
    exe = pathlib.Path(tmp) / 'delay'
    cpp.write_text('#include <initializer_list>\n' + support + function + driver)
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/delay.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 256 sequences of 80 calls')
