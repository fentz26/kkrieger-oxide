#!/usr/bin/env python3
"""Original GenMesh::Add and CollisionCube, with all indexed metadata."""
import pathlib
import subprocess
import tempfile
import verify_cube as cube
import verify_mesh as oracle

root=oracle.root
support=cube.support
support=support.replace('struct KOp{};', '''
struct DummyMaterial{int Id;void AddRef(){}void Release(){}};
DummyMaterial default_material{1},material_a{31},material_b{47};
struct GenMeshMtrl{DummyMaterial*Material;int JobIds[16],Pass,Remap;};
struct KOp{int Id;};KOp event_a{37},event_b{89};
''')
support=support.replace('Storage<GenMeshColl>Coll;','Storage<GenMeshMtrl>Mtrl;Storage<int>Lgts;Storage<GenMeshColl>Coll;int Pivot=-1;')
support=support.replace('VertBuf(storage.data()){}','VertBuf(storage.data()){Mtrl.AtLeast(2);Mtrl.Count=2;Mtrl[0].Material=nullptr;Mtrl[0].Pass=0;Mtrl[1].Material=&default_material;Mtrl[1].Pass=0;}')
start=support.index(' void Add(GenMesh*other)')
end=support.index(' void CopyFrom(',start)
support=support[:start]+''' bool Add(GenMesh*,int=0);int VertMask(){return 47;}void Realloc(int){}void UnPrepare(){}
'''+support[end:]
support+='\n#define sCopyMem memcpy\n'
functions=cube.functions+'\n'+oracle.function('sBool GenMesh::Add(')
writer=cube.writer+'''\nword(m.Coll.Count);for(int i=0;i<m.Coll.Count;i++){auto&cell=m.Coll[i];for(auto v:cell.Vert)word(v);word(cell.Mode);word(cell.Logic.Code);word(cell.Logic.Condition);word(cell.Logic.Value);word(cell.Logic.Output);for(auto*e:cell.Logic.Event)word(e?e->Id:0xffffffff);}
word(m.Mtrl.Count);for(int i=0;i<m.Mtrl.Count;i++){auto&b=m.Mtrl[i];word(b.Material?b.Material->Id:0);word(b.Pass);}
word(m.Lgts.Count);for(int i=0;i<m.Lgts.Count;i++)word(m.Lgts[i]);
word(m.Parts.Count);for(int i=0;i<m.Parts.Count;i++)word(m.Parts[i]);word(m.Pivot);
'''
driver='''
int main(){word(48);for(int c=0;c<48;c++){
 GenMesh m;int mode=c%32;int tx=1+c%3,ty=1+(c/3)%2,tz=1+(c/6)%4;
 auto*collision=Mesh_CollisionCube(nullptr,&event_a,&event_b,-1.25,0.75,-2.,1.5,-0.5,2.25,mode,tx,ty,tz,257+c,511-c,-71*c,769+c);
 m.Add(collision);delete collision;
 sFSRT srt={{{1,1,1},{0,0,0},{0,0,0}}};auto*other=Mesh_Cube(1+c%2,1,1+(c/2)%2,c%16,srt);
 other->Mtrl.Add();other->Mtrl[2].Material=&material_a;other->Mtrl[2].Pass=c%3;
 other->Mtrl.Add();other->Mtrl[3].Material=&material_b;other->Mtrl[3].Pass=c%2;
 for(int i=0;i<other->Face.Count;i++)other->Face[i].Material=i%4;
 other->Lgts.Add();other->Lgts[0]=other->Vert.Count-1;other->Pivot=2;
 for(int i=0;i<1+c%3;i++)m.Add(other);
 delete other;
'''+writer+'''}}
'''
def main():
    with tempfile.TemporaryDirectory() as tmp:
        cpp,exe=pathlib.Path(tmp)/'assembly.cpp',pathlib.Path(tmp)/'assembly'
        cpp.write_text(support+functions+cube.cube+driver)
        subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
        data=subprocess.check_output([str(exe)])
    out=root/'tests/fixtures/mesh-assembly.bin'
    out.write_bytes(data)
    print(f'{out}: 48 collision/assembly cases, materials, events, light slots and parts')

if __name__=='__main__':
    main()
