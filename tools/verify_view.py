#!/usr/bin/env python3
"""Native original InitView and DeleteChain state at scene transitions."""
import pathlib, subprocess, tempfile
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kdoc.cpp').read_text(encoding='cp1252')
functions='\n'.join(oracle.function(s) for s in ['void KInstanceMem::DeleteChain(', 'void KEnvironment::InitView('])
support=r'''
#include <cstdio>
#include <vector>
using sInt=int;
#define sCOUNTOF(a) (sizeof(a)/sizeof(*(a)))
int deleted=0;
struct KInstanceMem {KInstanceMem*Next=nullptr;char*DeleteArray=nullptr;KInstanceMem**DeleteChild=nullptr,**DeleteChild2=nullptr;void DeleteChain();~KInstanceMem(){deleted++;}};
struct sVector{float x,y,z,w;};
struct sMatrix{sVector i,j,k,l;void Init(){i={1,0,0,0};j={0,1,0,0};k={0,0,1,0};l={0,0,0,1};}};
template<class T>struct Array{std::vector<T>v;int Count=0;T operator[](int i){return v[i];}};
struct KEvent{KInstanceMem*FirstInstance;};struct Game{Array<int>Monsters;};
struct KEnvironment{Array<KEvent*>DynamicEvents;int ClearInstanceMem;KInstanceMem*DefaultInstance,*NextInstance,**LinkInstance;float Aspect;int BeatTime,CurrentTime,LastTime,TimeDelta,TimeSlices,TimeJitter,TimeReset;sMatrix Markers[32];struct Game*Game;void InitView();};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}
'''
driver=r'''
int main(){word(64);for(int c=0;c<64;c++){deleted=0;KEnvironment e{};Game game;e.Game=c%2?&game:nullptr;game.Monsters.Count=17;e.DefaultInstance=new KInstanceMem;e.NextInstance=e.DefaultInstance;e.LinkInstance=&e.DefaultInstance;e.Aspect=c+2;e.BeatTime=e.CurrentTime=e.LastTime=e.TimeDelta=e.TimeSlices=e.TimeJitter=c+99;e.TimeReset=0;
for(int i=0;i<c%5;i++)e.DynamicEvents.v.push_back(new KEvent{new KInstanceMem});e.DynamicEvents.Count=c%5;
e.InitView();word(e.DynamicEvents.Count);word(e.ClearInstanceMem);word(e.DefaultInstance==nullptr);word(e.NextInstance==nullptr);word(e.LinkInstance==nullptr);real(e.Aspect);for(int n:{e.BeatTime,e.CurrentTime,e.LastTime,e.TimeDelta,e.TimeSlices,e.TimeJitter,e.TimeReset})word(n);for(auto m:e.Markers)for(auto v:{m.i,m.j,m.k,m.l})for(float f:{v.x,v.y,v.z,v.w})real(f);word(e.Game?game.Monsters.Count:0);word(deleted);}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'view.cpp';exe=pathlib.Path(tmp)/'view';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/view.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 64 original view resets')
