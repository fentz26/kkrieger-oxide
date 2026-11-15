#!/usr/bin/env python3
"""Original MonsterMagnetAI and random-ball fallback, including the 256 cap."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger';oracle.source=(base/'kkriegergame.cpp').read_text(encoding='cp1252');function=oracle.function('void KKriegerGame::MonsterMagnetAI(')
oracle.source=(base/'_types.cpp').read_text(encoding='cp1252');random='\n'.join(oracle.function(s) for s in ['sU32 sGetRnd()','sF32 sFGetRnd()','void sSetRndSeed(','void sVector::InitRnd('])
support=r'''
#include <cstdio>
#include <cmath>
#include <vector>
using sU32=unsigned;using sInt=int;using sF32=float;
#define KMS_ACTIVE 4
#define sMOBILE 1
static unsigned sRandomSeed=0x74382381;
double sFSqrt(double x){return sqrt(x);}
struct sVector{float x,y,z,w;void Init(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}void Sub3(const sVector&a,const sVector&b){x=a.x-b.x;y=a.y-b.y;z=a.z-b.z;}float Dot3(const sVector&a){return x*a.x+y*a.y+z*a.z;}void InitRnd();void Unit3(){float scale=1./sqrt(double(Dot3(*this)));x*=scale;y*=scale;z*=scale;}void AddScale3(const sVector&v,float f){x+=v.x*f;y+=v.y*f;z+=v.z*f;}};
struct KKriegerMonster{int State;struct{sVector Pos;}Collider;sVector MagnetForce;};
template<class T>struct Array{std::vector<T>v;int Count=0;T Get(int i){return v[i];}void Add(T x){v.push_back(x);Count++;}};
struct KKriegerGame{Array<KKriegerMonster*>Monsters;void MonsterMagnetAI();};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vec(sVector v){real(v.x);real(v.y);real(v.z);real(v.w);}
'''
driver=r'''
int main(){word(64);for(int c=0;c<64;c++){
 int counts[]={0,1,2,8,32,257,300,512};int count=counts[c%8];int seed=c-32;word(count);word(seed);KKriegerGame game;std::vector<KKriegerMonster>monsters(count);
 for(int i=0;i<count;i++){
  auto&m=monsters[i];m.State=c%3==0?4:c%3==1?(i%3?4:2):i%5;m.Collider.Pos={float(i%7)*0.125f,float(i%3)*0.0625f,float(i/7)*0.125f,1};
  if(c%4==0)m.Collider.Pos={0,0,0,1};if(c%4==1 && i%3==0)m.Collider.Pos={0.0001f*i,0,0,1};m.MagnetForce={11,12,13,14};
  word(m.State);vec(m.Collider.Pos);game.Monsters.Add(&m);
 }
 sSetRndSeed(seed);game.MonsterMagnetAI();for(auto&m:monsters)vec(m.MagnetForce);word(sGetRnd());
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'magnet.cpp';exe=pathlib.Path(tmp)/'magnet';cpp.write_text(support+random+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/monster-magnet.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 64 original monster-force phases')
