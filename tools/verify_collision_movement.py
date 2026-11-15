#!/usr/bin/env python3
"""Original complete MoveCollider through generated faces and dynamic geometry."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
import verify_collision_ray as ray
import verify_collision_sphere as sphere

root = oracle.root
oracle.source = (root / '.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
functions = ray.functions + '\n' + '\n'.join(oracle.function(s) for s in [
    'void KKriegerGame::CollideSoftSphereFace(', 'sBool KKriegerGame::CheckSphereVsFace(',
    'sBool KKriegerGame::CheckSphereVsEdge(', 'sF32 KKriegerCell::approxDistance(',
    'sBool KKriegerGame::MoveCollider(', 'void KKriegerGame::CollideSoftSphereAdd(',
    'void KKriegerGame::CollideSoftSphereSub(', 'KKriegerCellAdd *KKriegerGame::FindCell(',
])
support = '#define sDPrintF(...)\n#define KPM_PLAYER 2\n#define KCRF_ISMONSTER 2\n' + ray.support
support = support.replace(' void Sub3(', '''
 float Abs3()const{return float(sqrt(double(Dot3(*this))));}
 void Unit3(){Scale3(float(1.0/sqrt(double(Dot3(*this)))));}
 void Scale3(const sVector&a,float f){x=a.x*f;y=a.y*f;z=a.z*f;}
 float Distance(const sVector&a)const{sVector v;v.Sub3(*this,a);return v.Abs3();}
 void Sub3(''')
support = support.replace('int OutsideMask(', 'float approxDistance(const sVector&);int OutsideMask(')
support = support.replace('struct KKriegerCellDynamic:KKriegerCell {};', 'struct KKriegerCellDynamic:KKriegerCell {void*Monster=nullptr;};')
support = support.replace('KKriegerCellDynamic*DCell;};', 'KKriegerCellDynamic*DCell;void Init(unsigned zone=0,unsigned flags=0){sSetMem(this,0,sizeof(*this));ZoneMask=zone;Flags=flags;}};')
support = support.replace('struct KKriegerGame {', '''
struct KKSolidCollider {sVector Pos;float Radius;KKriegerCellAdd*Cell;};
struct KKriegerGame {
 struct PlayerType {KKriegerCellDynamic HitCell;}Player;
 bool MoveCollider(KKSolidCollider&,const sVector&,int);
 void CollideSoftSphereAdd(sVector&,KKriegerCellAdd**,int&);
 void CollideSoftSphereSub(sVector&,KKriegerCell*);
 void CollideSoftSphereFace(sVector&,const sVector*,int);
 bool CheckSphereVsFace(const sVector&,const sVector*,int,sVector&);
 bool CheckSphereVsEdge(const sVector&,const sVector&,const sVector&,sVector&);
 KKriegerCellAdd*FindCell(const sVector&);
''')
driver = r'''
void inputcell(KKriegerCell&cell,int id,int mode,float x,float sx,float sy,float sz){
 sVector v[8];for(int k=0;k<8;k++)v[k]={x-((k&1)-0.5f)*sx,-(((k>>1)&1)-0.5f)*sy,-(((k>>2)&1)-0.5f)*sz,1};
 word(mode);for(auto p:v)vector(p);cell.Init(v,mode);cell.Id=id;
}
int main(){word(64);word(20);for(int c=0;c<64;c++){
 KKriegerGame game;KKriegerCellAdd adds[2];KKriegerCell other[2];KKriegerCellDynamic moving;
 word(4);word(2);inputcell(adds[0],0,0,0,6,4,4);inputcell(adds[1],1,0,5,6,4,4);
 inputcell(other[0],2,1,1+(c%3)*0.5f,0.5f,2,2);inputcell(other[1],3,2,1,3,3,3);
 inputcell(game.Player.HitCell,4,1,0,1,2,1);word(0);
 inputcell(moving,5,c%3==0?2:1,1.5f,0.5f,2,2);moving.Monster=c%2?(void*)1:nullptr;word(c%2);
 game.DCellUsed=2;game.DCellPtr[0]=&game.Player.HitCell;game.DCellPtr[1]=&moving;
 for(auto&cell:adds){*game.CellAdd.Add()=&cell;*game.CellList.Add()=&cell;}for(auto&cell:other)*game.CellList.Add()=&cell;
 game.CellConnect();for(auto cell:game.CellAdd.v)game.CreateCollisionFaces(*cell);
 for(int k=0;k<20;k++){
  KKSolidCollider collider{{(k%9-2)*0.5f,(k%4-1)*0.5f,((k+c)%3-1)*0.25f,1},(c%3+1)*0.25f,nullptr};
  collider.Cell=game.FindCell(collider.Pos);if(!collider.Cell)collider.Cell=&adds[0];
  sVector move={((k+c)%7-3)*0.25f,((k+c/7)%3-1)*0.125f,(k%3-1)*0.125f,0};int who=k%4;
  vector(collider.Pos);real(collider.Radius);word(collider.Cell->Id);vector(move);word(who);
  word(game.MoveCollider(collider,move,who));vector(collider.Pos);real(collider.Radius);word(collider.Cell->Id);
  for(auto cell:game.CellList.v){word(cell->Enter);word(cell->Leave);word(cell->Collided);}for(int i=0;i<2;i++){auto cell=game.DCellPtr[i];word(cell->Enter);word(cell->Leave);word(cell->Collided);}
 }
 for(auto&cell:adds)cell.DeleteFaces();
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'movement.cpp'
    exe = pathlib.Path(tmp) / 'movement'
    cpp.write_text(support + functions + driver)
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/collision-movement.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 1280 original collider movements')
