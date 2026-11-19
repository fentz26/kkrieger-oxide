#!/usr/bin/env python3
"""Original Mesh_Cube, including tessellation, UV creases and collision cells.

Only the source's x86 stack-parameter indexing (&tx)[j] is made ABI-portable.
The collision-only Add adapter rejects geometry and preserves original reindexing.
"""
import pathlib
import subprocess
import tempfile
import sys
import struct
import verify_mesh as oracle

root = oracle.root
support = oracle.support[:oracle.support.index('int main()')]
support = support.replace('#define sGMI_UV0 4', '''#define sGMI_UV0 5
#define sGMI_POS 0
#define sGMI_COLOR0 3
#define sGMF_DEFAULT 47
#define sGMF_ALL 511
#define sGMF_POS 1
#define sGMF_UV0 32
#define sGMF_NORMAL 2
#define sGMI_NORMAL 1
#define sGMI_TANGENT 2
#define MAS_EDGE 1
#define MAS_FACE 2
#define MAS_VERT 4
#define sTRUE true
#define sFALSE false
#define sLINK_KKRIEGER 1
#define sPLAYER 1
#define __stdcall
float sFAbs(float f){return fabsf(f);}
// Source sFSinCos executes x87 fsincos, then stores its two float results.
// Host sinf/cosf approximations can differ before those single-precision stores.
void sFSinCos(float f,float&s,float&c){s=float(sin(double(f)));c=float(cos(double(f)));}
void sSetMem4(uint32_t*p,uint32_t v,int n){for(int i=0;i<n;i++)p[i]=v;}
''')
support = support.replace('struct sVector{float x=0,y=0,z=0,w=0;};', '''struct sVector{float x=0,y=0,z=0,w=0;
 void Init(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}
 void Init3(float a,float b,float c){x=a;y=b;z=c;}
 void Sub3(const sVector&a,const sVector&b){x=a.x-b.x;y=a.y-b.y;z=a.z-b.z;}
 float Dot3(const sVector&a)const{return x*a.x+y*a.y+z*a.z;}
 void Cross3(const sVector&a,const sVector&b){x=a.y*b.z-a.z*b.y;y=a.z*b.x-a.x*b.z;z=a.x*b.y-a.y*b.x;}
 void AddScale3(const sVector&a,float f){x+=a.x*f;y+=a.y*f;z+=a.z*f;}
 void Add3(const sVector&a){x+=a.x;y+=a.y;z+=a.z;}
 void Scale3(const sVector&a,float f){x=a.x*f;y=a.y*f;z=a.z*f;}
 void Unit3(){Scale3(1.0f/sqrtf(Dot3(*this)));}
 void UnitSafe3(){float e=Dot3(*this);if(e>1e-20f)Scale3(1.0f/sqrtf(e));else Init3(1,0,0);}

 void Init4(float a,float b,float c,float d){Init(a,b,c,d);}
 void Scale3(float f){x*=f;y*=f;z*=f;}
};
struct sMatrix{sVector i,j,k,l;void Init(){i.Init(1,0,0,0);j.Init(0,1,0,0);k.Init(0,0,1,0);l.Init(0,0,0,1);}void InitEulerPI2(const float*);void InitSRT(const float*);};
struct Triple{float x,y,z;};union sFSRT{struct{Triple s,r,t;};float v[9];};
struct KOp{};struct KLogic{uint8_t Code,Condition,Output,pad;int Value;KOp*Event[2];};
struct GenMeshColl{int Vert[8],Mode;KLogic Logic;};
''')
support = support.replace('Storage<GenMeshFace>Face;', 'Storage<GenMeshFace>Face;Storage<GenMeshColl>Coll;Storage<int>Parts;')
support = support.replace('int VertMap(int n){return n;}', 'int VertMap(int n){return n==5?4:n>=0&&n<4?n:-1;}')
support = support.replace('void Extrude(sU32,int,int);', 'void Extrude(sU32,int=0,int=0);')
support = support.replace('void Ring(int,sU32,float,float,int);','''void Ring(int,sU32,float,float,int=0);
 sVector&VertPos(int i){return VertBuf[i*5];}void Init(int,int){}void Release(){delete this;}
 void Add(GenMesh*other){assert(other->Face.Count==0 && other->Edge.Count==0);int offset=Vert.Count;
 for(int i=0;i<other->Vert.Count;i++){int v=AddNewVert();Vert[v]=other->Vert[i];Vert[v].Next+=offset;Vert[v].First+=offset;Vert[v].ReIndex+=offset;CopyFrom(v,other,i);}
 for(int i=0;i<other->Coll.Count;i++){Coll.Add();Coll[Coll.Count-1]=other->Coll[i];for(auto&v:Coll[Coll.Count-1].Vert)v+=offset;}}
 void CopyFrom(int v,GenMesh*other,int i){for(int j=0;j<5;j++)VertBuf[v*5+j]=other->VertBuf[i*5+j];}
 void CalcNormals(int,int);void Mask2Sel(sU32);void All2Sel(bool=true,int=7);void Sel2Mask(sU32,int=0);void Face2Vert();void Vert2FaceEdge();
 void SelectCube(const sVector&,const sVector&,sU32,int);void TransVert(sMatrix&,int=0,int=0);void Crease(int=0,sU32=0,int=0,int=254);
''')
support += '''
bool CheckMesh(GenMesh*,int=0){return false;}
GenMesh*Mesh_CollisionCube(GenMesh*,KOp*,KOp*,float,float,float,float,float,float,int,int,int,int,int=0,int=0,int=0,int=0);
'''
functions = oracle.functions + '\n' + '\n'.join(oracle.function(s) for s in ['void GenMesh::Mask2Sel(', 'void GenMesh::All2Sel(', 'void GenMesh::Sel2Mask(', 'void GenMesh::Face2Vert(', 'void GenMesh::Vert2FaceEdge(', 'void GenMesh::SelectCube(', 'void GenMesh::TransVert(', 'void GenMesh::Crease(', 'void GenMesh::CalcNormals(', 'GenMesh * __stdcall Mesh_CollisionCube('])
cube = oracle.function('GenMesh * __stdcall Mesh_Cube(').replace('(&tx)[j]', 'tess[j]').replace('sInt i,j;', 'sInt i,j;const int tess[3]={tx,ty,tz};')
types = (root/'.reference/fr_public/werkkzeug3_kkrieger/_types.cpp').read_text(encoding='cp1252')
original_source = oracle.source
oracle.source = types
functions += '\n' + oracle.function('void sMatrix::InitEulerPI2(') + '\n' + oracle.function('void sMatrix::InitSRT(')
oracle.source = original_source
# Reuse the complete state writer from the ring oracle.
writer = oracle.support[oracle.support.index(' word(m.Vert.Count);'):oracle.support.rindex('}}')]
driver = r'''
int main(){word(128);for(int c=0;c<128;c++){
 int tx=1+c%3,ty=1+(c/3)%3,tz=1+(c/9)%3;int flags=(c%16)|((c/16)%4)*16|((c/64)%2)*64;
 sFSRT srt;for(int i=0;i<3;i++){srt.v[i]=(1+(c+i)%5)*0.25f;srt.v[i+3]=c%2?0.03125f*(c%9-i):0;srt.v[i+6]=0.125f*(i-c%7);}
 word(tx);word(ty);word(tz);word(flags);for(auto v:srt.v)real(v);
 auto*p=Mesh_Cube(tx,ty,tz,flags,srt);auto&m=*p;
'''+writer+r'''
 word(m.Coll.Count);for(int i=0;i<m.Coll.Count;i++){auto&cell=m.Coll[i];for(int v:cell.Vert)word(v);word(cell.Mode);word(cell.Logic.Code);word(cell.Logic.Condition);word(cell.Logic.Value);word(cell.Logic.Output);}
 word(m.Parts.Count);for(int i=0;i<m.Parts.Count;i++)word(m.Parts[i]);delete p;
}}
'''
def main():
    global driver
    suffix = ""
    if "--development" in sys.argv or "--beta" in sys.argv:
        sys.path.insert(0,str(root/".reference/wasm"))
        import kxread
        kxread.round=lambda value,places:value
        beta="--beta" in sys.argv
        document=kxread.read((root/(".reference/verified-beta.kx" if beta else "assets/kkrieger3383.kx")).read_bytes(),beta=beta)
        ops=[o for o in document["ops"] if o["cid"]==0x81]
        words=[]
        for op in ops:
            par=op["par"]
            words.append([*par[:4],*[struct.unpack("<I",struct.pack("<f",f))[0] for f in par[4:13]]])
        table="const uint32_t cases[][13]={"+",".join("{"+",".join(str(v)+"u" for v in p)+"}" for p in words)+"};"
        start=driver.index("int main()")
        body=driver.index(" word(tx);",start)
        driver=table+"\nint main(){word("+str(len(words))+");for(int c=0;c<"+str(len(words))+";c++){const auto&params=cases[c];int tx=params[0],ty=params[1],tz=params[2],flags=params[3];sFSRT srt;memcpy(srt.v,params+4,36);\n"+driver[body:]
        suffix="-beta" if beta else "3383"
    if "--normals" in sys.argv:
        driver=driver.replace("auto&m=*p;", "auto&m=*p;for(int i=0;i<m.Face.Count;i++)m.Face[i].Select=(i+c)%3!=0;m.CalcNormals(c%2,1+c%7);")
        suffix+="-normals"
    with tempfile.TemporaryDirectory() as tmp:
        cpp, exe = pathlib.Path(tmp)/'cube.cpp', pathlib.Path(tmp)/'cube'
        cpp.write_text(support+functions+'\n'+cube+driver)
        subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
        data = subprocess.check_output([str(exe)])
    out=root/(('.reference/mesh-cube'+suffix+'.bin') if suffix.startswith('-beta') else ('tests/fixtures/mesh-cube'+suffix+'.bin'))
    out.write_bytes(data)
    print(f'{out}: original cubes, including topology, UVs, SRT, collision and part IDs')

if __name__=='__main__':
    main()
