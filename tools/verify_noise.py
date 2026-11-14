#!/usr/bin/env python3
"""Generate noise samples from original Farbrausch scalar source, offline only."""
import os
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parent.parent
ref = root / ".reference"
types = (ref / "fr_public/werkkzeug3_kkrieger/_types.cpp").read_text(errors="replace")
bitmap = (ref / "fr_public/werkkzeug3_kkrieger/genbitmap.cpp").read_text(errors="replace")
rng = types[types.index("static sU32 sRandomSeed"):types.index("sInt sMakePower2")]
init = types[types.index("static const sChar GTable"):types.index("#define ix")]
noise = bitmap[bitmap.index("static sF32 noise2("):bitmap.index("GenBitmap * __stdcall Bitmap_Perlin")]
prefix = r'''
#include <cstdint>
#include <cstring>
#include <iostream>
#include <utility>
using sU32=uint32_t;using sU8=uint8_t;using sInt=int32_t;using sF32=float;using sChar=char;
#define sMOBILE 1
#define sSwap std::swap
#define sCopyMem memcpy
struct sVector {float x,y,z,w;};
sVector sPerlinGradient3D[16];
sU8 sPerlinPermute[512];
sF32 sPerlinRandom[256][2];
'''
driver = r'''
void word(uint32_t v){std::cout.write((char*)&v,4);}
int main(){
 sInitPerlin();uint32_t state=0x1234abcd;
 auto next=[&](){state^=state<<13;state^=state>>17;state^=state<<5;return state;};
 word(1000);
 for(int i=0;i<1000;i++){
  int32_t x=next(),y=next(),mask=(1<<(i%9))-1,seed=next()&255;
  float value=noise2(x,y,mask,seed);uint32_t bits;memcpy(&bits,&value,4);
  word(x);word(y);word(mask);word(seed);word(bits);
 }
}
'''
cpp = ref / "noise-oracle.cpp"
cpp.write_text(prefix + rng + init + noise + driver)
exe = ref / "noise-oracle"
subprocess.run([os.environ.get("CXX", "clang++"), "-std=c++17", "-O2", "-fwrapv",
                "-ffp-contract=off", str(cpp), "-o", str(exe)], check=True)
out = root / "tests/fixtures/noise.bin"
out.parent.mkdir(exist_ok=True)
with out.open("wb") as stream:
    subprocess.run([str(exe)], stdout=stream, check=True)
print(out)
