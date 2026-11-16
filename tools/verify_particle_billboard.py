#!/usr/bin/env python3
"""Extract the original PartSystem billboard loop and compare native vertices."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle

root = oracle.root
base = root / '.reference/fr_public/werkkzeug3_kkrieger'
oracle.source = (base / 'geneffect.cpp').read_text(encoding='cp1252')
setvert = oracle.function('void SetVert(sF32 *fp,')
function = oracle.function('void __stdcall Exec_Effect_PartSystem(')
body = function[function.index('        middle ='):function.index('        /*for(si=')]
oracle.source = (base / 'genbitmap.cpp').read_text(encoding='cp1252')
color = oracle.function('sU64 GetColor64(sU32 c)')
support = r'''
#include <cstdio>
#include <cmath>
#include <cstdint>
#include <algorithm>
#include <initializer_list>
using sInt=int;using sF32=float;using sU32=uint32_t;using sU64=uint64_t;
struct sVector{float x,y,z,w;void AddScale3(const sVector&v,float s){x+=v.x*s;y+=v.y*s;z+=v.z*s;}};
struct Particle{sVector Pos,Speed;int Time,MaxTime;};
struct{ sVector i,j; } mat;
struct{float x,y,z;} size;
int sDivShift(int a,int b){return int((int64_t(a)*65536)/b);}
float sFade(float a,float b,float f){return a+(b-a)*f;}
void sFSinCos(float a,float&s,float&c){s=float(sin(double(a)));c=float(cos(double(a)));}
// Scalar translation of the original MMX Fade64, already covered by bitmap fixtures.
void Fade64(sU64&r,sU64&a,sU64&b,int f){int wb=std::clamp(f>>1,-32768,32767),wa=std::clamp(32768-(f>>1),-32768,32767);r=0;for(int k=0;k<4;k++){int x=int16_t(a>>(k*16)),y=int16_t(b>>(k*16));uint16_t v=uint16_t((((x*wa)>>16)+((y*wb)>>16))*2);r|=sU64(v)<<(k*16);}}
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
'''
driver = r'''
int main(){word(256);for(int n=0;n<256;n++){
Particle p{{n*.125f,-n*.25f,n*.0625f,(n-128)*.03125f},{},n%129,129};Particle*part=&p;
unsigned col0=0xff123456u+unsigned(n)*65537,col1=0x80765432u+unsigned(n)*257,col2=0x40012345u+unsigned(n)*17;
float fmiddle=(n%15+1)/16.f,ffade=(n%7)/8.f;size={.125f+n*.03125f,.25f+n*.015625f,.5f+n*.0625f};
mat.i={1,.125f,-.25f,0};mat.j={-.5f,1,.25f,0};
for(auto c:{col0,col1,col2})word(c);for(float f:{size.x,size.y,size.z,fmiddle,ffade,p.Pos.x,p.Pos.y,p.Pos.z,p.Pos.w})real(f);word(p.Time);word(p.MaxTime);for(float f:{mat.i.x,mat.i.y,mat.i.z,mat.j.x,mat.j.y,mat.j.z})real(f);
col0=(col0&0xff00ff00)|((col0&0x00ff0000)>>16)|((col0&0x00ff)<<16);
col1=(col1&0xff00ff00)|((col1&0x00ff0000)>>16)|((col1&0x00ff)<<16);
col2=(col2&0xff00ff00)|((col2&0x00ff0000)>>16)|((col2&0x00ff)<<16);
sU64 c0=GetColor64(col0),c1=GetColor64(col1),c2=GetColor64(col2),cr,cf,black=0xffff000000000000ULL;
sInt middle,percent,fade,fadetime;sF32 s,ss,sc;sVector v;float vertices[32],*fp=vertices;
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'billboard.cpp'
    exe = pathlib.Path(tmp) / 'billboard'
    cpp.write_text(support + color + setvert + driver + body + 'fwrite(vertices,4,32,stdout);}}')
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/particle-billboard.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 256 original billboard evaluations')
