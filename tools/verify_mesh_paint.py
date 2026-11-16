#!/usr/bin/env python3
"""Original BuildPaintJobs mesh-pass body with deterministic material key input."""
import pathlib
import subprocess
import tempfile
import verify_paint_culling as culling
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/engine.cpp').read_text(encoding='cp1252')
function=oracle.function('void Engine_::BuildPaintJobs()')
body=function[function.index('        sInt usage = pass->Usage;'):function.rfind('\n      }')]
# A 64-bit host pointer is not the original 32-bit pass allocation identity.
body=body.replace('sInt materialId = sInt(pass) >> 3;', 'sInt materialId = MaterialKey;')
support=r'''
#include <vector>
#define ENG_MAXPASS 256
#define ENGU_SHADOW 3
#define ENGU_LIGHT 4
#define EL_SHADOW 2
#define sMBF_STENCILZFAIL 4
#define MAXLIGHT LightCapacity
int sRange(int x,int high,int low){return std::clamp(x,low,high);}
struct sMaterial{};
struct EngMaterialInsert{int GetPriority(){return 1;}};
struct GenMaterialPass{int Usage,Pass;sMaterial*Mtrl;};
struct EngLight{sVector Position;float Range;int Flags;sFrustum SVFrustum,ZFailCull;};
struct EngMesh{struct MeshJob{sAABox BBox;int AnimMatrix;}Jobs[1];void*Animation;};
struct PaintJob{unsigned SortKey;int JobId;EngMesh*Mesh;int MatrixId;GenMaterialPass*Pass;EngLight*Light;int Flags;void*BoneData;};
struct JobArray{std::vector<PaintJob*>v;PaintJob**Add(){v.push_back(nullptr);return &v.back();}}PaintJobs;
struct Allocator{template<class T>T*Alloc(){return new T;}}Mem;
struct MeshMaterial{int JobIds[1];int Pass;};struct MeshSceneJob{int MatrixId,PassAdjust;};
unsigned UsageMask;int MaterialKey,LightCapacity,LightJobCount;EngLight LightJobs[16];EngMaterialInsert*Inserts[256];sMatrix Matrices[1];sFrustum Frustum;
void run(EngMesh*mesh,MeshMaterial*meshMat,MeshSceneJob*job,GenMaterialPass*pass,EngMaterialInsert*insert){int j=0,animBase=-1;void*boneData=nullptr;do{
'''
driver=r'''
}while(false);}
void plane(sCullPlane&p,float w){p.Plane={0,0,0,w};p.CalcNVert();}
int main(){word(512);for(int c=0;c<512;c++){
int usage=c%8,animated=(c/8)%2,visible=(c/16)%2;
UsageMask=(c/32)%3==0?~0u:1u<<((c/8)%8);MaterialKey=(c*31337)&65535;LightCapacity=(c/8)%2?4:16;LightJobCount=4;
int pass=(c*17)%300,meshPass=(c*7)%511-255,adjust=(c*13)%511-255;
for(int n:{usage,animated,visible,int(UsageMask),MaterialKey,LightCapacity,pass,meshPass,adjust})word(n);
EngMesh mesh{};mesh.Jobs[0].BBox={{-1,-1,-1,1},{1,1,1,1}};mesh.Jobs[0].AnimMatrix=-1;mesh.Animation=animated?&mesh:nullptr;
Matrices[0]={{1,0,0,0},{0,1,0,0},{0,0,1,0},{0,0,0,1}};
for(auto&p:Frustum.Planes)plane(p,visible?1.f:-1.f);
for(int i=0;i<4;i++){auto&light=LightJobs[i];float distance=(c+i)%3==0?20.f:0.f;light.Position={distance,0,0,1};light.Range=2;light.Flags=(c/2+i)%2?2:0;for(auto&p:light.SVFrustum.Planes)plane(p,(c/4+i)%2?1.f:-1.f);for(auto&p:light.ZFailCull.Planes)plane(p,(c/8+i)%2?1.f:-1.f);light.ZFailCull.NPlanes=(c+i)%7;
for(float f:{light.Position.x,light.Position.y,light.Position.z,light.Range})real(f);word(light.Flags);real(light.SVFrustum.Planes[0].Plane.w);word(light.ZFailCull.NPlanes);real(light.ZFailCull.Planes[0].Plane.w);}
MeshMaterial material{{0},meshPass};MeshSceneJob job{0,adjust};GenMaterialPass gp{usage,pass,nullptr};EngMaterialInsert insert;
for(auto&p:Inserts)p=nullptr;run(&mesh,&material,&job,&gp,&insert);
int inserted=-1;for(int i=0;i<256;i++)if(Inserts[i])inserted=i;
word(inserted);word(PaintJobs.v.size());for(auto*p:PaintJobs.v){word(p->SortKey);word(p->Light?int(p->Light-LightJobs):-1);word(p->Flags!=0);delete p;}PaintJobs.v.clear();}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp=pathlib.Path(tmp)/'paint.cpp';exe=pathlib.Path(tmp)/'paint'
    cpp.write_text(culling.support+culling.functions+support+body+driver)
    subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/mesh-paint.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 512 original mesh-pass expansion calls')
