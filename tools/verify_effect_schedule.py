#!/usr/bin/env python3
"""Original effect BuildPaintJobs and stable radix sort, without a GPU."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
root = oracle.root
oracle.source = (root / '.reference/fr_public/werkkzeug3_kkrieger/engine.cpp').read_text(encoding='cp1252')
build = oracle.function('void Engine_::BuildPaintJobs()')
build = build[:build.index('  // process meshes')] + '}\n'
sort = oracle.function('void Engine_::SortPaintJobs()')
support = r'''
#include <cstdio>
#include <cstring>
#include <algorithm>
using sInt=int;using sU32=unsigned;
#define sZONE(x)
#define sVERIFY(x) ((void)0)
#define ENG_MAXPASS 256
#define sTRUE true
#define sSetMem std::memset
int sRange(int x,int high,int low){return std::clamp(x,low,high);}
struct GenEffect{int Pass,Usage;bool NeedCurrentRender=false;};
struct KOp{void* Cache;};
struct Engine_{
struct EffectJob{EffectJob*Next;KOp*Op;int PassAdjust,Id;};
struct PaintJob{unsigned SortKey;int JobId;void*Light;EffectJob*EffJob;};
struct Array{PaintJob**Array;int Count=0,Alloc=4096;};
};
'''.replace('struct Array{PaintJob**Array;int Count=0,Alloc=4096;};',r'''
struct Jobs{PaintJob**Array=new PaintJob*[4096];int Count=0,Alloc=4096;PaintJob*&operator[](int i){return Array[i];}PaintJob**Add(){return Array+Count++;}void AtLeast(int){}void Swap(Jobs&j){std::swap(Array,j.Array);std::swap(Count,j.Count);std::swap(Alloc,j.Alloc);}~Jobs(){delete[]Array;}};
struct Allocator{template<class T>T*Alloc(){return new T;}}Mem;
EffectJob*EffectJobs=nullptr;Jobs PaintJobs,PaintJobs2;bool NeedCurrentRender[256]{};
void BuildPaintJobs();void SortPaintJobs();
''')
driver = r'''
void word(unsigned n){fwrite(&n,4,1,stdout);}
int main(){word(32);word(64);for(int c=0;c<32;c++){Engine_ engine;GenEffect effects[64];KOp ops[64];Engine_::EffectJob jobs[64];
for(int i=0;i<64;i++){int kind=(i+c)%3==0?0x61:0x64, present=(i+c)%7!=0,pass=(i*19+c*13)%300,usage=(i+c)%8,adjust=(i*7+c*31)%511-128;
for(int n:{kind,present,pass,usage,adjust})word(n);
effects[i]={present?pass:0,present&&kind==0x61?usage:7,false};ops[i]={&effects[i]};jobs[i]={engine.EffectJobs,&ops[i],adjust,i};engine.EffectJobs=&jobs[i];}
engine.BuildPaintJobs();engine.SortPaintJobs();for(int i=0;i<64;i++){word(engine.PaintJobs[i]->EffJob->Id);word(engine.PaintJobs[i]->SortKey);delete engine.PaintJobs[i];}}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'schedule.cpp'
    exe = pathlib.Path(tmp) / 'schedule'
    cpp.write_text(support + build + sort + driver)
    subprocess.run(['clang++','-std=c++17','-O2',str(cpp),'-o',str(exe)],check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/effect-schedule.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 32 original effect builds and stable sorts')
