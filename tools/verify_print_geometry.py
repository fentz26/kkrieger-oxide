#!/usr/bin/env python3
"""Original EffectPrintInfo and Print alignment loop, without font rasterization."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/geneffect.cpp').read_text(encoding='cp1252')
methods='\n'.join(oracle.function(s) for s in ['void SetVert(sF32 *fp,','void EffectPrintInfo::Print(','sF32 EffectPrintInfo::Width(sInt chr)','sF32 EffectPrintInfo::Width(sChar *text,'])
start=oracle.source.index('struct EffectPrintInfo\n')
structure=oracle.source[start:oracle.source.index('\n};',start)+3]
function=oracle.function('void __stdcall Exec_Effect_Print(')
loop=function[function.index('      switch((flags&0x300)>>8)'):function.index("      while(*text=='\\n')")]
support=r'''
#include <cstdio>
#include <cmath>
#include <cstdint>
#include <initializer_list>
using sInt=int;using sF32=float;using sU16=uint16_t;using sU32=uint32_t;using sU64=uint64_t;using sChar=char;
struct UV{float x0,y0,x1,y1;float XSize(){return x1-x0;}float YSize(){return y1-y0;}};
struct KLetterMetric{struct UV UV;float PreSpace,Width;};struct KEnvironment{KLetterMetric Letters[8][256]{};};
double sFSin(double f){return sin(f);}double sFCos(double f){return cos(f);}
#define sQuad(ip,a,b,c,d) {ip[0]=a;ip[1]=b;ip[2]=c;ip[3]=a;ip[4]=c;ip[5]=d;ip+=6;}
void SetVert(float*,float,float,float,float,float,sU64=~sU64(0));
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
'''
driver=r'''
int main(){word(256);for(int c=0;c<256;c++){KEnvironment env;EffectPrintInfo pi{};pi.KEnv=&env;pi.Page=0;pi.Max=4096;
int flags=(c%4)<<8,centerchar=c%3==0?c%5:-1;float radius=c%3==0?0.f:(c%2==0?-2.f:3.f),y=(c-128)*.0625f;
pi.XSize=.125f+(c%9)*.25f;pi.YSize=.25f+(c%7)*.125f;pi.SpaceX=(c%5-2)*.03125f;pi.SpaceY=0;
word(flags);word(centerchar);for(float f:{pi.XSize,pi.YSize,pi.SpaceX,pi.SpaceY,radius,y})real(f);
int codes[]={65,66,128,32};for(int k=0;k<4;k++){auto&l=env.Letters[0][codes[k]];l.UV={k*.125f,.125f,k*.125f+(k+1)*.03125f,.125f+(k==3?0.f:.5f)};l.PreSpace=(k-2)*.0078125f;l.Width=(k+2)*.0625f;for(float f:{l.UV.x0,l.UV.y0,l.UV.x1,l.UV.y1,l.PreSpace,l.Width})real(f);}
char buffer[16]{};int len=c%10;word(len);for(int i=0;i<len;i++){buffer[i]=char(codes[(i+c)%4]);word(uint8_t(buffer[i]));}
float vertices[4096*32];sU16 indices[4096*6];pi.fp=vertices;pi.ip=indices;float x=0,spacex=pi.SpaceX;char*p=buffer;
'''
ending=r'''
real(pi.Width(buffer));word(pi.Count*4);word(pi.Count*6);fwrite(vertices,4,pi.Count*32,stdout);for(int i=0;i<pi.Count*6;i++)word(indices[i]);}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp=pathlib.Path(tmp)/'print.cpp';exe=pathlib.Path(tmp)/'print'
    cpp.write_text(support+structure+methods+driver+loop+ending)
    subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/print-geometry.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 256 original Print layouts')
