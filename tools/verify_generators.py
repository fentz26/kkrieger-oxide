#!/usr/bin/env python3
"""Original scalar generator outputs for all published parameter sets.

Uses reduced dimensions for the Perlin/Gradient fixtures. Function bodies come
from the pinned reference source. Fade64's scalar harness is separately checked
against original MMX by verify_bitmap.py. No reference C++ ships in the game.
"""
import os
import pathlib
import struct
import subprocess
import sys
from oracle_mm import MMX

root = pathlib.Path(__file__).resolve().parent.parent
ref = root / ".reference"
sys.path.insert(0, str(ref / "wasm"))
import kxread
kxread.round = lambda value, places: value  # retain exact decoded float bits
beta = "--beta" in sys.argv
data_path = next((p for p in sys.argv[1:] if not p.startswith("--")),".reference/verified-beta.kx" if beta else "assets/kkrieger3383.kx")
data = kxread.read((root / data_path).read_bytes(),beta=beta)
ids = (0x22,0x27,0x29,0x2B,0x2C,0x2D,0x2E,0x30,0x32,0x34,0x36,0x37,0x38,0x39)
cases = [(i, o) for i, o in enumerate(data["ops"]) if o["cid"] in ids]
fixture = bytearray(struct.pack("<I", len(cases)))
for i, op in cases:
    par = list(op["par"])
    if op["cid"] in (0x22,0x32,0x34):
        par[0] = par[1] = 5
    pack = data["classes"][op["cls"]][2]
    fixture.extend(struct.pack("<III", i, op["cid"], len(par)))
    for fmt, value in zip(pack.lower(), par):
        if fmt in "gfe":
            fixture.extend(struct.pack("<f", value))
        else:
            fixture.extend(struct.pack("<I", int(value, 16) if isinstance(value, str) else value & 0xFFFFFFFF))
input_path = ref / "generator-input.bin"
input_path.write_bytes(fixture)
source = (ref / "fr_public/werkkzeug3_kkrieger/genbitmap.cpp").read_text(errors="replace")
types = (ref / "fr_public/werkkzeug3_kkrieger/_types.cpp").read_text(errors="replace")
def extract(text, start, end):
    return text[text.index(start):text.index(end)]
rng = extract(types, "static sU32 sRandomSeed", "sInt sMakePower2")
init = extract(types, "static const sChar GTable", "#define ix")
color = extract(source, "sU64 GetColor64(sU32 c)", "sU32 GetColor32")
gamma = extract(source, "static sInt GammaTable[1025]", "/***   Operators")
# Remove the decorative comment fragment following GetGamma.
gamma = gamma[:gamma.rfind("/****************************************************************************/")]
hscb = extract(source, "GenBitmap * __stdcall Bitmap_HSCB(", "GenBitmap * __stdcall Bitmap_Wavelet(")
perlin = extract(source, "static sF32 noise2(", "GenBitmap * __stdcall Bitmap_Cell(")
gradient = extract(source, "GenBitmap * __stdcall Bitmap_Gradient(", "GenBitmap * __stdcall Bitmap_Sharpen(")
glow = extract(source, "GenBitmap * __stdcall Bitmap_GlowRect(", "GenBitmap * __stdcall Bitmap_Dots(")
cell = extract(source, "GenBitmap * __stdcall Bitmap_Cell(", "GenBitmap * __stdcall Bitmap_Gradient(")
spatial = extract(source, "GenBitmap * __stdcall Bitmap_Rotate(", "GenBitmap * __stdcall Bitmap_Light(")
bump = extract(source, "GenBitmap * __stdcall Bitmap_Bump(", "struct letterz")
blur = extract(source, "GenBitmap * __stdcall Bitmap_Blur(", '#pragma optimize("",on)').replace('__asm { emms };', '')
color_op = extract(source, "GenBitmap * __stdcall Bitmap_Color(", "GenBitmap * __stdcall Bitmap_GlowRect(")
sharpen = extract(source, "GenBitmap * __stdcall Bitmap_Sharpen(", "GenBitmap * __stdcall Bitmap_Import(")
prefix = r'''
#include <algorithm>
#include <cassert>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iostream>
#include <utility>
#include <vector>
#include <cstdarg>
using sU32=uint32_t;using sU8=uint8_t;using sU16=uint16_t;using sU64=uint64_t;
using sInt=int32_t;using sF32=float;using sChar=char;using sBool=bool;using sS32=int32_t;
#define sMOBILE 1
#define sINTRO 1
#define sFALSE false
#define sTRUE true
#define KC_BITMAP 1
#define sVERIFY(x) assert(x)
#define __stdcall
#define sSwap std::swap
#define sCopyMem memcpy
#define NOTEXTURES0()
#define NOTEXTURES1(x)
#define NOTEXTURES2(x,y)
#define SCRIPTVERIFY(x) assert(x)
const double sPI2=6.28318530717958647692;
const float sPI2F=6.28318530717958647692f;
const float sPIF=3.14159265358979323846f;
double sFPow(double a,double b){return pow(a,b);}
double sFSin(double a){return sin(a);}double sFCos(double a){return cos(a);}
double sFSqrt(double a){return sqrt(a);}double sFInvSqrt(double a){return 1/sqrt(a);}
void sFSinCos(float a,float&s,float&c){s=sin(double(a));c=cos(double(a));}
float sFAbs(float a){return fabsf(a);}
int sFtol(float a){return (int)nearbyintf(a);}
template<class T>T sMax(T a,T b){return std::max(a,b);}
template<class T>T sMin(T a,T b){return std::min(a,b);}
int sMulShift(int a,int b){return ((int64_t)a*b)>>16;}
int sDivShift(int a,int b){return ((int64_t)a<<16)/b;}
int sMulDiv(int a,int b,int c){return int64_t(a)*b/c;}
template<class T>T sRange(T a,T hi,T lo){return std::clamp(a,lo,hi);}
int sGetPower2(int x){int i=0;while((1<<i)<x)i++;return i;}
int sRange7fff(int x){return std::clamp(x,0,32767);}
struct sVector {float x,y,z,w;};
sVector sPerlinGradient3D[16];sU8 sPerlinPermute[512];sF32 sPerlinRandom[256][2];
struct GenBitmap {
 int XSize=0,YSize=0,Size=0,ClassId=1,RefCount=1,Format=1,TexMipCount=0,TexMipTresh=0;uint64_t*Data=nullptr;
 void Init(int x,int y){XSize=x;YSize=y;Size=x*y;Data=new uint64_t[Size]();}
 void AddRef(){RefCount++;}void Release(){if(--RefCount==0)delete this;}
 GenBitmap*Copy(){auto*b=new GenBitmap;b->Init(XSize,YSize);memcpy(b->Data,Data,Size*8);return b;}
 ~GenBitmap(){delete[] Data;}
};
GenBitmap*NewBitmap(int x,int y){auto*b=new GenBitmap;b->Init(1<<std::clamp(x,0,12),1<<std::clamp(y,0,12));return b;}
bool CheckBitmap(GenBitmap*&b,GenBitmap**in=nullptr){if(!b)return true;auto*old=b;if(in)*in=b;if(b->RefCount>1){if(in){b=new GenBitmap;b->Init(old->XSize,old->YSize);}else b=old->Copy();old->Release();}return false;}
int16_t sat(int64_t v){return std::clamp<int64_t>(v,-32768,32767);}
void Fade64(uint64_t&r,uint64_t&a,uint64_t&b,int fade){
 int16_t wa=sat(32768-(fade>>1)),wb=sat(fade>>1);auto aa=a,bb=b;uint64_t out=0;
 for(int i=0;i<4;i++){
  int16_t x=(aa>>(i*16))&65535,y=(bb>>(i*16))&65535;
  uint16_t v=uint16_t((int(x)*wa>>16)+(int(y)*wb>>16))<<1;
  out|=uint64_t(v)<<(i*16);
 }r=out;
}
'''
blend = r'''
enum{BI_ADD=0,BI_SUB=1,BI_MUL=2,BI_DIFF=3,BI_ALPHA=4,BI_MULCOL=5,BI_ADDCOL=6,BI_SUBCOL=7,BI_GRAY=8,BI_INVERT=9,BI_SCALECOL=10,BI_MERGE=11,BI_BRIGHTNESS=12,BI_SUBR=13,BI_MULMERGE=14,BI_SHARPEN=15,BI_HARDLIGHT=16,BI_OVER=17,BI_ADDSMOOTH=18,BI_RANGE=19};
int GenBitmapTextureSizeOffset=0;
GenBitmap*Bitmap_Merge(int mode,int count,GenBitmap*b0,...){
 if(mode==5)mode=12;else if(mode>=6&&mode<=8)mode+=10;
 if(count==1)return b0;GenBitmap*in;CheckBitmap(b0,&in);
 va_list args;va_start(args,b0);for(int i=1;i<count;i++){auto*b=va_arg(args,GenBitmap*);assert(b->Size==b0->Size);Bitmap_Inner(b0->Data,b->Data,b0->Size,mode,in->Data);b->Release();in=b0;}va_end(args);return b0;
}
'''
driver = r'''
uint32_t read(std::istream&in){uint32_t v;in.read((char*)&v,4);if(!in)abort();return v;}
void word(uint32_t v){std::cout.write((char*)&v,4);}
void pixels(GenBitmap*b){std::cout.write((char*)b->Data,b->Size*8);}
void verify_primitives(const char*dir){
 std::ifstream in(std::string(dir)+"/bitmap-inner.bin",std::ios::binary);
 auto count=read(in);while(count--){int mode=read(in),n=read(in);std::vector<uint64_t>d(n),s(n),x(n),expected(n);for(auto*v:{&d,&s,&x,&expected})in.read((char*)v->data(),n*8);Bitmap_Inner(d.data(),s.data(),n,mode,x.data());assert(d==expected);}
 std::ifstream samples(std::string(dir)+"/bitmap-bilinear.bin",std::ios::binary);
 auto n=read(samples),w=read(samples),h=read(samples);std::vector<uint64_t>image(w*h);samples.read((char*)image.data(),image.size()*8);
 while(n--){int u=read(samples),v=read(samples),border=read(samples);uint64_t expected=read(samples);expected|=uint64_t(read(samples))<<32;BilinearContext ctx;BilinearSetup(&ctx,image.data(),w,h,border);uint64_t actual;BilinearFilter(&ctx,&actual,u,v);assert(actual==expected);}
}
int main(int argc,char**argv){
 if(argc!=3)return 2;verify_primitives(argv[2]);sInitPerlin();std::ifstream in(argv[1],std::ios::binary);
 auto n=read(in);word(n);
 while(n--){
  auto id=read(in),cid=read(in),count=read(in);std::vector<uint32_t> p(count);for(auto&w:p)w=read(in);
  auto f=[&](int i){float v;memcpy(&v,&p[i],4);return v;};
  word(id);word(cid);word(count);for(auto w:p)word(w);
  GenBitmap*b=nullptr;
  if(cid==0x22){b=Bitmap_Perlin(p[0],p[1],p[2],p[3],f(4),p[5],p[6],f(7),f(8),p[9],p[10]);word(b->XSize);word(b->YSize);word(0);}
  if(cid==0x34){b=Bitmap_Gradient(p[0],p[1],p[2],p[3],f(4),f(5),f(6),p[7]);word(b->XSize);word(b->YSize);word(0);}
  if(cid==0x32){b=Bitmap_Cell(p[0],p[1],p[2],p[3],p[4],p[5],p[6],f(7),f(8),p[9],f(10),p[11],f(12));word(b->XSize);word(b->YSize);word(0);}
  if(!b){
   b=new GenBitmap;b->Init(cid==0x2b?8:16,cid==0x2b?4:8);b->RefCount=2;auto*w=(uint16_t*)b->Data;
   for(int i=0;i<b->Size;i++){w[i*4]=(i*131+27)&32767;w[i*4+1]=(i*977+13441)&32767;w[i*4+2]=(i*421+1007)&32767;w[i*4+3]=(i*37+32000)&32767;}
   GenBitmap*other=nullptr;
   if(cid==0x2d||cid==0x30){other=b->Copy();other->RefCount=2;auto*w=(uint16_t*)other->Data;for(int i=0;i<b->Size;i++){w[i*4]=(i*377+6011)&32767;w[i*4+1]=(i*231+15031)&32767;w[i*4+2]=(i*421+30713)&32767;w[i*4+3]=32767;}}
   word(b->XSize);word(b->YSize);word(b->Size*(other?2:1));pixels(b);if(other)pixels(other);
   if(cid==0x2b)b=Bitmap_HSCB(b,f(0),f(1),f(2),f(3));
   if(cid==0x27||cid==0x39)b=Bitmap_GlowRect(b,f(0),f(1),f(2),f(3),f(4),f(5),p[6],f(7),f(8),p[9],p[10]);
   if(cid==0x29)b=Bitmap_Blur(b,p[0],f(1),f(2),f(3));
   if(cid==0x2c)b=Bitmap_Rotate(b,f(0),f(1),f(2),f(3),f(4),p[5],p.size()>6?p[6]:0,p.size()>7?p[7]:0);
   if(cid==0x2d)b=Bitmap_Distort(b,other,f(0),p[1]);
   if(cid==0x2e)b=Bitmap_Normals(b,f(0),p[1]);
   if(cid==0x30)b=Bitmap_Bump(b,other,p[0],f(1),f(2),f(3),f(4),f(5),p[6],p[7],f(8),f(9),f(10),p[11],f(12),f(13));
   if(cid==0x36)b=Bitmap_RotateMul(b,f(0),f(1),f(2),f(3),f(4),p[5],p[6],p[7],p[8],p[9]);
   if(cid==0x37)b=Bitmap_Twirl(b,f(0),f(1),f(2),f(3),f(4),f(5),p[6]);
   if(cid==0x38)b=Bitmap_Sharpen(b,p[0],f(1),f(2),f(3));
  }
  pixels(b);delete b;
 }
}
'''
def common_source():
    return prefix + rng + init + color + gamma + MMX + blend + color_op + glow + hscb + blur + spatial + bump + perlin + cell + gradient + sharpen

def main():
    cpp = ref / "generator-oracle.cpp"
    cpp.write_text(common_source() + driver)
    exe = ref / "generator-oracle"
    subprocess.run([os.environ.get("CXX", "clang++"), "-std=c++17", "-O2", "-fwrapv",
                    "-ffp-contract=off", str(cpp), "-o", str(exe)], check=True)
    output = ref / "generators-beta.bin" if beta else root / "tests/fixtures/generators3383.bin"
    temporary = ref / "generator-results.bin"
    with temporary.open("wb") as stream:
        subprocess.run([str(exe), str(input_path),str(root / "tests/fixtures")], stdout=stream, check=True)
    output.write_bytes(temporary.read_bytes())
    print(output)

if __name__ == "__main__":
    main()
