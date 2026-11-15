#!/usr/bin/env python3
"""Original connected-cell ray traversal and persistent static/dynamic zones."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
import verify_collision_world as world

root = oracle.root
oracle.source = (root / '.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
functions = world.functions + '\n' + '\n'.join(oracle.function(s) for s in [
    'sBool KKriegerCell::RejectPlane(', 'sInt KKriegerCell::OutsideMask(',
    'sBool KKriegerCell::IntersectOut(', 'sBool KKriegerCell::IntersectIn(',
    'void KKriegerGame::CellSiftDown(', 'sBool KKriegerGame::CollideRay(',
    'sBool KKriegerGame::CollideRaySub(', 'sBool KKriegerGame::CollideRaySub2(',
])
connect = oracle.function('void KKriegerGame::CellConnect(')
functions += '\n' + connect[:connect.index('  // now generate')] + '\n}\n'
support = '#define GAMEPERF(x)\n#define LOGGING 0\n#define KCRF_ISPLAYER 1\n' + world.support
support = support.replace(' void Sub3(', '''
 void Neg4(){x=-x;y=-y;z=-z;w=-w;}
 void Sub4(const sVector&a,const sVector&b){x=a.x-b.x;y=a.y-b.y;z=a.z-b.z;w=a.w-b.w;}
 void AddScale3(const sVector&a,float f){x+=a.x*f;y+=a.y*f;z+=a.z*f;}
 void Sub3(''')
support = support.replace('void Init(const sVector*,int);', '''
unsigned Enter,Leave,Collided;int Id;
void Init(const sVector*,int);bool RejectPlane(const sVector&,const sVector&);
int OutsideMask(const sVector&,float=0);bool IntersectOut(const sVector&,const sVector&,sVector&,float&,float=0);
bool IntersectIn(const sVector&,const sVector&,sVector&,float&,float=0);
''')
support = support.replace('struct KKriegerGame {', '''
template<class T>void sSwap(T&a,T&b){std::swap(a,b);}
struct KKriegerCellDynamic:KKriegerCell {};
struct KKriegerCollideInfo {sVector Plane,Pos;float Dist,Radius;unsigned ZoneMask,Flags;KKriegerCellDynamic*DCell;};
struct KKriegerGame {
 int DCellUsed=0;KKriegerCellDynamic*DCellPtr[512];
 void CellSiftDown(int,int);void CellConnect();
 bool CollideRay(KKriegerCellAdd*&,const sVector&,const sVector&,KKriegerCollideInfo&);
 bool CollideRaySub(const sVector&,const sVector&,const sVector&,KKriegerCellAdd**,int,KKriegerCollideInfo&);
 bool CollideRaySub2(const sVector&,const sVector&,const sVector&,KKriegerCell*,KKriegerCollideInfo&);
''')
driver = r'''
void inputcell(KKriegerCell&cell,int id,int mode,float x,float y,float z,float sx,float sy,float sz){
 sVector v[8];for(int k=0;k<8;k++)v[k]={x-((k&1)-0.5f)*sx,y-(((k>>1)&1)-0.5f)*sy,z-(((k>>2)&1)-0.5f)*sz,1};
 word(mode);for(auto p:v)vector(p);cell.Init(v,mode);cell.Id=id;
}
void info(KKriegerCollideInfo&ci){vector(ci.Plane);vector(ci.Pos);real(ci.Dist);real(ci.Radius);word(ci.ZoneMask);word(ci.Flags);word(ci.DCell?ci.DCell->Id:~0u);}
int main(){word(64);word(20);for(int c=0;c<64;c++){
 KKriegerGame game;KKriegerCellAdd adds[18];KKriegerCell other[4];KKriegerCellDynamic dynamic[2];word(22);word(2);
 for(int i=0;i<18;i++){inputcell(adds[i],i,0,i*2.f,0,0,2,4,4);*game.CellList.Add()=&adds[i];*game.CellAdd.Add()=&adds[i];}
 for(int i=0;i<4;i++){inputcell(other[i],18+i,i%2?2:1,(i*2+c%5)*0.5f,0,0,i%2?3.f:0.5f,2,2);*game.CellList.Add()=&other[i];}
 inputcell(dynamic[0],22,1,(c%7)*0.5f,0,0,0.5f,2,2);
 inputcell(dynamic[1],23,2,(c%5)*0.5f,0,0,2,2,2);
 game.DCellUsed=2;game.DCellPtr[0]=&dynamic[0];game.DCellPtr[1]=&dynamic[1];game.CellConnect();
 for(int k=0;k<20;k++){
  int initial=k%3==0?0:k%3==1?5:10;KKriegerCellAdd*add=&adds[initial];
  sVector p0={initial*2.f,0,0,1};static float dx[10]={0,0.5f,1,2,4,10,25,40,-6,-30};
  sVector p1={p0.x+dx[k%10],(k%5-2)*0.25f,((k+c)%3-1)*0.25f,1};
  KKriegerCollideInfo ci{{1,2,3,4},{5,6,7,8},0.17f,(c%3)*0.125f,1u<<(k%7),static_cast<unsigned>(c%4),k%2?&dynamic[1]:nullptr};
  word(initial);vector(p1);vector(p0);info(ci);word(game.CollideRay(add,p1,p0,ci));word(add->Id);info(ci);
  for(auto cell:game.CellList.v){word(cell->Enter);word(cell->Leave);word(cell->Collided);}for(auto&cell:dynamic){word(cell.Enter);word(cell.Leave);word(cell.Collided);}
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'ray.cpp'
    exe = pathlib.Path(tmp) / 'ray'
    cpp.write_text(support + functions + driver)
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/collision-ray.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 1280 original world ray queries')
