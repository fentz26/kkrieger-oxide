#!/usr/bin/env python3
"""Original player UpdateTexture conversion/mips with memory-only D3D locks."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_start.cpp').read_text()
old=oracle.source;oracle.source=source
function=oracle.function('void sSystem_::UpdateTexture(sInt handle,sU16 *source,');oracle.source=old
support=r'''
#include <vector>
#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
using sInt=int;using sU32=uint32_t;using sU16=uint16_t;using sU8=uint8_t;
#define sINTRO 1
#define sTF_Q8W8V8U8 5
#define sTIF_RENDERTARGET 4
#define FAILED(x) ((x)<0)
void sCopyMem4(sU32*d,sU32*s,int n){memcpy(d,s,n*4);}
void word(unsigned n){fwrite(&n,4,1,stdout);}int sGetPower2(int n){int p=1;while((1<<p)<n)p++;return p;}
struct D3DLOCKED_RECT{void*pBits;int Pitch;};
struct Level{int w,h;bool written=false;std::vector<unsigned char>pixels;};
struct IDirect3DTexture9{std::vector<Level>levels;int LockRect(int i,D3DLOCKED_RECT*lock,int,int){if(i>=(int)levels.size())return -1;auto&l=levels[i];l.written=true;lock->pBits=l.pixels.data();lock->Pitch=l.w*4;return 0;}void UnlockRect(int){}};
struct sHardTex{IDirect3DTexture9*Tex;int XSize,YSize,MipLevels,Format,Flags=0;};
struct sSystem_{sHardTex Textures[1];void UpdateTexture(int,sU16*,int);};
'''
driver=r'''
int main(){word(256);for(unsigned c=0;c<256;c++){
 int width=1<<(c%5),height=1<<((c/5)%5);if(c>=240){width=5;height=3;}
 int format=c&1?5:1;int counts[]={0,1,2,4,99};int thresholds[]={0,1,2,15,16,17,31,32,47};int requested=counts[(c/11)%5],threshold=thresholds[(c/7)%9];
 int count=std::min(requested,std::min(sGetPower2(width),sGetPower2(height)));if((width&(width-1))||(height&(height-1)))count=1;
 IDirect3DTexture9 texture;int levels=count?count:1;for(int n=std::max(width,height);!count&&n>1;n>>=1)levels++;
 for(int i=0;i<levels;i++){int w=std::max(1,width>>i),h=std::max(1,height>>i);texture.levels.push_back({w,h,false,std::vector<unsigned char>(w*h*4)});}
 sSystem_ system;system.Textures[0]={&texture,width,height,count,format,0};std::vector<sU16>input(width*height*4);
 for(unsigned i=0;i<input.size();i++)input[i]=(i*38473u+c*18763u)&0xffff;
 word(width);word(height);word(format);word(requested);word(threshold);fwrite(input.data(),2,input.size(),stdout);
 system.UpdateTexture(0,input.data(),threshold);word(levels);int written=0;for(auto&l:texture.levels)written+=l.written;word(written);
 for(auto&l:texture.levels)if(l.written){word(l.w);word(l.h);for(size_t i=0;i<l.pixels.size();i+=4){auto*p=l.pixels.data()+i;if(format==1){putchar(p[2]);putchar(p[1]);putchar(p[0]);putchar(p[3]);}else fwrite(p,1,4,stdout);}}
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'upload.cpp';exe=pathlib.Path(tmp)/'upload'
 cpp.write_text(support+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-fno-strict-aliasing',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/texture-upload.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes / 256 original texture conversions')
