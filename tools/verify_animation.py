#!/usr/bin/env python3
"""Compile the original VM as an offline oracle, never linked into the Rust game.

Requires pinned references from fetch_reference.py and a C++ compiler. The three
original evaluation functions are extracted verbatim; the scaffold only provides
their platform-independent types and fixture transport. The output is regression
data, not C++ runtime code. Farbrausch license: third_party/farbrausch-LICENSE.txt.
"""
import os
import pathlib
import subprocess
import sys

root = pathlib.Path(__file__).resolve().parent.parent
ref = root / ".reference"
upstream = ref / "fr_public" / "werkkzeug3_kkrieger"
source = (upstream / "kdoc.cpp").read_text(errors="replace")
header = (upstream / "kdoc.hpp").read_text(errors="replace")
defines = "\n".join(line for line in header.splitlines()
                    if line.startswith("#define KA_") or line.startswith("#define KV_MATRIX_")
                    or line.startswith("#define KSI_") or line.startswith("#define KK_MAXVAR"))

def function(start, end):
    return source[source.index(start):source.index(end)].rstrip()

vm = function("#define STACKMAX 256", "sInt CalcCmdSize")
splines = function("sF32 KSplineChannel::Eval", "void KEvent::Init")

scaffold = r'''
#include <algorithm>
#include <cassert>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iostream>
#include <vector>
using sU8=uint8_t; using sU16=uint16_t; using sU32=uint32_t;
using sS32=int32_t; using sF32=float; using sInt=int; using sBool=bool;
#define sPLAYER 1
#define sTRUE true
#define sVERIFY(x) do { if(!(x)) {fprintf(stderr,"oracle assertion at %d\n",__LINE__); abort();} } while(0)
#define sFatal(x) do {fprintf(stderr,"%s\n",x);abort();} while(0)
const float sPI2F=6.28318530717958647692f;
double sFSin(double x){return sin(x);} double sFCos(double x){return cos(x);}
double sFMod(double x,double y){return fmod(x,y);}
template<class T>T sRange(T x,T hi,T lo){return std::max(lo,std::min(hi,x));}
struct sMatrix;
struct sVector {
 float x=0,y=0,z=0,w=0;
 void Init(){x=y=z=w=0;}
 void Add4(const sVector &v){x+=v.x;y+=v.y;z+=v.z;w+=v.w;}
 void Sub4(const sVector &v){x-=v.x;y-=v.y;z-=v.z;w-=v.w;}
 void Mul4(const sVector &v){x*=v.x;y*=v.y;z*=v.z;w*=v.w;}
 void Rotate4(const sMatrix&);
};
struct sMatrix{sVector i,j,k,l;};
void sVector::Rotate4(const sMatrix&m){auto v=*this;for(int r=0;r<4;r++)(&x)[r]=(&m.i.x)[r]*v.x+(&m.j.x)[r]*v.y+(&m.k.x)[r]*v.z+(&m.l.x)[r]*v.w;}
uint8_t sPerlinPermute[512]={};
struct KSplineKey{float Time,Value;};
struct KSplineChannel{int KeyCount=0;KSplineKey*Keys=nullptr;float Eval(float,int);};
struct KSpline{KSplineChannel Channel[4];int Count=0,Interpolation=0;void Eval(float,sVector&);};
struct KOp{
 uint32_t *DataEdit,*DataAnim;int words,WheelSpeed=0,SkipCalc=0;
 int GetDataWords(){return words;}void UpdateVar(int,int){}
};
struct KEnvironment{
 int Index=0; sVector Stack[256];uint8_t StackAdr[256];sVector Var[32];
 KSpline*EventSpline=nullptr;KSpline**Splines=nullptr;
 int ExecuteAnim(KOp*,uint8_t*);
};
'''
driver = r'''
uint32_t read(std::istream&in){uint32_t v;in.read((char*)&v,4);if(!in)abort();return v;}
float real(std::istream&in){auto v=read(in);float f;memcpy(&f,&v,4);return f;}
void write(uint32_t v){std::cout.write((char*)&v,4);}
int main(int argc,char**argv){
 if(argc!=2)return 2;
 std::ifstream in(argv[1],std::ios::binary);
 std::vector<KSpline> splines(read(in));
 for(auto&s:splines){s.Interpolation=read(in);s.Count=read(in);assert(s.Count<=4);
  for(int c=0;c<s.Count;c++){auto&ch=s.Channel[c];ch.KeyCount=read(in);ch.Keys=new KSplineKey[ch.KeyCount];
   for(int k=0;k<ch.KeyCount;k++){ch.Keys[k].Time=real(in);ch.Keys[k].Value=real(in);}
  }
 }
 struct Program{uint32_t id;std::vector<uint32_t> words;std::vector<uint8_t> code;};
 std::vector<Program> programs(read(in));
 for(auto&p:programs){p.id=read(in);p.words.resize(read(in));for(auto&w:p.words)w=read(in);
  p.code.resize(read(in));in.read((char*)p.code.data(),p.code.size());if(!in)abort();
 }
 write(5);write(programs.size());
 for(float time:{0.f,0.25f,1.f,17.375f,1024.f})for(auto&p:programs){
  KEnvironment env;KSpline*base=splines.data();env.Splines=&base;
  for(int i=0;i<4;i++){(&env.Var[0].x)[i]=time;(&env.Var[1].x)[i]=0.75f;(&env.Var[28+i].x)[i]=1;}
  std::vector<uint32_t> result=p.words;
  KOp op{p.words.data(),result.data(),(int)p.words.size()};
  int saves=env.ExecuteAnim(&op,p.code.data());
  write(p.id);write(result.size());write(op.WheelSpeed==255);write(saves);
  for(auto w:result)write(w);
 }
}
'''
cpp = ref / "animation-oracle.cpp"
cpp.write_text(defines + "\n" + scaffold + "\n" + splines + "\n" + vm + "\n" + driver)
exe = ref / "animation-oracle"
subprocess.run([os.environ.get("CXX", "clang++"), "-std=c++17", "-O2",
                "-ffp-contract=off", str(cpp), "-o", str(exe)], check=True)
path = sys.argv[1] if len(sys.argv) > 1 else "assets/kkrieger3383.kx"
flags = ["--beta"] if "--beta" in sys.argv else []
fixture = ref / "animation-input.bin"
with fixture.open("wb") as stream:
    subprocess.run(["cargo", "run", "--quiet", "--bin", "kx-inspect", "--", path,
                    *flags, "--animation-fixture"], cwd=root, stdout=stream, check=True)
output = ref / "animation-beta.bin" if flags else root / "tests" / "fixtures" / "animation3383.bin"
output.parent.mkdir(exist_ok=True)
with output.open("wb") as stream:
    subprocess.run([str(exe), str(fixture)], stdout=stream, check=True)
print(output)
