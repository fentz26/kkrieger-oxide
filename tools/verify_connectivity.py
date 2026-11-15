#!/usr/bin/env python3
"""Original heap ordering and collision-cell connectivity before face generation."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle

root = oracle.root
oracle.source = (root / '.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
functions = oracle.function('void KKriegerGame::CellSiftDown(')
connect = oracle.function('void KKriegerGame::CellConnect(')
# Test the complete sorting/connection phase independently of face clipping.
functions += '\n' + connect[:connect.index('  // now generate')] + '\n}\n'
support = r'''
#include <vector>
#include <cstdio>
#include <cassert>
#include <utility>
#include <initializer_list>
using sInt=int;using sF32=float;
#define sVERIFY assert
#define KCM_ADD 0
#define KCM_SUB 1
#define KCM_ZONE 2
template<class T>void sSwap(T&a,T&b){std::swap(a,b);}
template<class T>struct Array {std::vector<T>v;int Count=0;T&operator[](int i){return v[i];}T*Add(){v.push_back(T{});Count=v.size();return &v.back();}};
struct Vec{float x,y,z,w;};struct KKriegerCell {Vec BBMin,BBMax;int Mode,Id;};
struct KKriegerCellAdd:KKriegerCell {Array<KKriegerCellAdd*>Adds;Array<KKriegerCell*>Subs,Zones;};
struct KKriegerGame {Array<KKriegerCell*>CellList;void CellSiftDown(int,int);void CellConnect();};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
'''
driver = r'''
int main(){word(128);for(int c=0;c<128;c++){
 KKriegerGame game;int count=c%65;word(count);std::vector<KKriegerCellAdd>cells(count);
 for(int k=0;k<count;k++){
  auto&cell=cells[k];cell.Id=k;cell.Mode=(k+c)%4;cell.BBMin={float((k*7+c)%9-4),float((k*3+c)%7-3)*0.5f,float((k*5+c)%11-5)*0.25f,1};
  cell.BBMax={cell.BBMin.x+(k%5)*0.5f,cell.BBMin.y+((k+c)%5)*0.25f,cell.BBMin.z+((k+c/3)%7)*0.125f,1};
  word(cell.Mode);for(auto v:{cell.BBMin,cell.BBMax})for(float f:{v.x,v.y,v.z,v.w})real(f);*game.CellList.Add()=&cell;
 }
 game.CellConnect();for(auto p:game.CellList.v)word(p->Id);
 for(auto&cell:cells){word(cell.Adds.Count);for(auto p:cell.Adds.v)word(p->Id);word(cell.Subs.Count);for(auto p:cell.Subs.v)word(p->Id);word(cell.Zones.Count);for(auto p:cell.Zones.v)word(p->Id);}
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'connectivity.cpp'
    exe = pathlib.Path(tmp) / 'connectivity'
    cpp.write_text(support + functions + driver)
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/collision-connectivity.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 128 original connectivity graphs')
