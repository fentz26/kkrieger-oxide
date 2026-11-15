#!/usr/bin/env python3
"""Original SetSceneR/AddMesh traversal, cell order and attached event placement."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
import verify_collision_faces as faces

root = oracle.root
base = root / '.reference/fr_public/werkkzeug3_kkrieger'
oracle.source = (base / 'kkriegergame.cpp').read_text(encoding='cp1252')
functions = faces.functions + '\n' + '\n'.join(oracle.function(s) for s in [
    'void KKriegerGame::SetSceneR(', 'void KKriegerGame::AddMesh(',
])
oracle.source = (base / '_types.cpp').read_text(encoding='cp1252')
functions += '\n' + '\n'.join(oracle.function(s) for s in [
    'void sMatrix::InitEulerPI2(', 'void sMatrix::InitSRT(', 'void sMatrix::MulA(',
    'void sVector::Rotate34(const sMatrix &m,const sVector &v)',
])
support = faces.support.replace('using sInt=int;', 'using sU32=unsigned;using sInt=int;')
support = support.replace('struct sVector {', 'struct sMatrix;\nstruct sVector {')
support = support.replace(' void Init3(){', '''
 void Init4(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}
 void Init3(float a,float b,float c){Init4(a,b,c,1);}
 void Scale4(const sVector&a,float f){x=a.x*f;y=a.y*f;z=a.z*f;w=a.w*f;}
 void AddScale4(const sVector&a,float f){x+=a.x*f;y+=a.y*f;z+=a.z*f;w+=a.w*f;}
 void Add4(const sVector&a){x+=a.x;y+=a.y;z+=a.z;w+=a.w;}
 void Rotate34(const sMatrix&,const sVector&);
 void Init3(){''')
support = support.replace('int Count=0;', 'int Count=0;void Init(){v.clear();Count=0;}')
support = support.replace('T&operator[](int i){return v[i];}', 'T&operator[](int i){return v[i];}T Get(int i){return v[i];}')
position = support.index('struct KKriegerCell {')
support = support[:position] + r'''
#define sPLAYER 1
#define sINTRO 1
#define sLINK_KKRIEGER 1
#define KCM_ADD 0
#define KCM_ZONE 2
const float sPI2F=6.2831853071795864769f;
float sFAbs(float x){return fabsf(x);}void sFSinCos(float x,float&s,float&c){s=sinf(x);c=cosf(x);}
struct sMatrix {sVector i,j,k,l;void Init(){i={1,0,0,0};j={0,1,0,0};k={0,0,1,0};l={0,0,0,1};}
 const sVector&operator[](int n)const{switch(n){case 0:return i;case 1:return j;case 2:return k;default:return l;}}
 void InitEulerPI2(const float*);void InitSRT(const float*);void MulA(const sMatrix&,const sMatrix&);
};
struct KOp {int Id;};
struct KLogic {unsigned char Code,Condition,Output;int Value;KOp*Event[2];};
struct KEvent {KOp*Op=nullptr;sVector Translate;float CullDist=0;void Init(){Translate={0,0,0,0};}};
struct KKriegerMeshCell {sVector Vert[8];int Mode;KLogic Logic;};
struct KKriegerMesh {int CellCount;KKriegerMeshCell*Cell;};
struct GenScene {float SRT[9];int Count=0;bool IsSector=false;int Id;Array<GenScene*>Childs;KKriegerMesh*CollMesh=nullptr;};
''' + support[position:]
support = support.replace('sVector Vertices[8];void Init(const sVector*,int);', 'sVector Vertices[8];GenScene*Scene;KLogic Logic;KEvent*Event[2];void Init(const sVector*,int);')
support = support.replace('Array<KKriegerCellAdd*>Adds;', 'Array<KKriegerCellAdd*>Adds;Array<KKriegerCell*>Subs,Zones;')
support = support.replace('struct KKriegerGame {', '''struct KKriegerGame {
 Array<KKriegerCellAdd*>CellAdd;Array<KKriegerCell*>CellSub,CellZone,CellList;
 void SetSceneR(GenScene*,const sMatrix&,GenScene*,int);void AddMesh(KKriegerMesh*,const sMatrix&,GenScene*);
''')
driver = r'''
int main(){word(64);for(int c=0;c<64;c++){
 KOp ops[2]={{71},{72}};KKriegerMeshCell mc[3];KKriegerMesh mesh{3,mc};
 word(3);for(int n=0;n<3;n++){
  auto&cell=mc[n];for(int k=0;k<8;k++)cell.Vert[k]={-((k&1)-0.5f)*(2+n),-(((k>>1)&1)-0.5f)*2,-(((k>>2)&1)-0.5f)*2,1};
  cell.Mode=n;cell.Logic={static_cast<unsigned char>(n+1),static_cast<unsigned char>(c%7),static_cast<unsigned char>(c%13),c-31,{n?&ops[0]:nullptr,n==2?&ops[1]:nullptr}};
  word(cell.Mode);for(auto v:cell.Vert)vector(v);word(cell.Logic.Code);word(cell.Logic.Condition);word(cell.Logic.Value);word(cell.Logic.Output);for(auto op:cell.Logic.Event)word(op?op->Id:~0u);
 }
 GenScene nodes[4];word(4);for(int n=0;n<4;n++){
  auto&node=nodes[n];node.Id=n;for(int i=0;i<9;i++)node.SRT[i]=i<3?1.f:0.f;
  node.SRT[0]=1+(n%2)*0.25f;node.SRT[6]=n*2+(c%3)*0.25f;node.SRT[7]=(n+c%4)*0.25f;node.SRT[8]=(c%5-2)*0.25f;
  node.Count=n==1?(c%5-1):0;node.IsSector=n==2||(n==3&&c%2==0);node.CollMesh=&mesh;
 }
 *nodes[0].Childs.Add()=&nodes[1];*nodes[0].Childs.Add()=&nodes[2];*nodes[1].Childs.Add()=&nodes[3];*nodes[2].Childs.Add()=&nodes[3];
 for(auto&node:nodes){for(float f:node.SRT)real(f);word(node.Count);word(node.IsSector);word(node.Childs.Count);for(auto child:node.Childs.v)word(child->Id);}
 sMatrix matrix;matrix.Init();KKriegerGame game;game.SetSceneR(&nodes[0],matrix,&nodes[0],0);word(game.CellList.Count);
 for(auto cell:game.CellList.v){word(cell->Mode);word(cell->OnlyForShot);word(cell->Scene?cell->Scene->Id:~0u);word(cell->PlaneCount);for(int i=0;i<cell->PlaneCount;i++)vector(cell->Planes[i]);for(auto v:cell->Vertices)vector(v);vector(cell->Center);vector(cell->BBMin);vector(cell->BBMax);
  word(cell->Logic.Code);word(cell->Logic.Condition);word(cell->Logic.Value);word(cell->Logic.Output);for(auto op:cell->Logic.Event)word(op?op->Id:~0u);
  for(auto event:cell->Event){word(event!=nullptr);if(event){word(event->Op->Id);vector(event->Translate);real(event->CullDist);delete event;}}
 }
 for(auto cell:game.CellAdd.v)delete cell;for(auto cell:game.CellSub.v)delete cell;for(auto cell:game.CellZone.v)delete cell;
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp = pathlib.Path(tmp) / 'world.cpp'
    exe = pathlib.Path(tmp) / 'world'
    cpp.write_text(support + functions + driver)
    subprocess.run(['clang++', '-std=c++17', '-O2', '-ffp-contract=off', str(cpp), '-o', str(exe)], check=True)
    data = subprocess.check_output([str(exe)])
out = root / 'tests/fixtures/collision-world.bin'
out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 64 original scene traversals')
