#!/usr/bin/env python3
"""Original C++ composed geometry outputs for the implemented operator subset.

Unsupported/animated branches are excluded explicitly. Material/event pointers
are stable identities; their rendering and execution belong to separate ports.
Cube/Add x86 stack varargs and Grid's adjacent-member component indexing
are adapted to explicit indexed arguments, preserving legacy layout semantics.
"""
import pathlib
import hashlib
import json
import struct
import subprocess
import sys
import tempfile
import verify_cube as cube
import verify_mesh as oracle
import verify_assembly as assembly
import verify_deform as deform
import portable_perlin3 as perlin3
from fixture_codec import pack as pack_fixture

root=oracle.root
sys.path.insert(0,str(root/'.reference/wasm'))
import kxread
kxread.round=lambda value,places:value
beta='--beta' in sys.argv
document=kxread.read((root/('.reference/verified-beta.kx' if beta else 'assets/kkrieger3383.kx')).read_bytes(),beta=beta)
ids={0x81,0x82,*range(0x85,0x8a),0x8c,0x8f,0x90,0x91,0xa0,0xa5,0xa6,0xa7,0xab,0xad,0xaf,0x92,0x93,0x95,0x96,0x9a,0x9c,0x9d,0xb0,0xb3}
command=[sys.executable,str(root/'tools/verify_texture_graph.py'),'--displacement-inputs']
if beta:command.append('--beta')
subprocess.run(command,check=True)
pixel_path=root/('.reference/displacement-pixels-beta.bin' if beta else '.reference/displacement-pixels3383.bin')
pixel_bytes=pixel_path.read_bytes();pixel_count=struct.unpack_from('<I',pixel_bytes)[0];position=4;pixel_ids=set()
for _ in range(pixel_count):
    i,w,h,_,_,_=struct.unpack_from('<6I',pixel_bytes,position);pixel_ids.add(i);position+=24+w*h*8
assert position==len(pixel_bytes)
def geometry_inputs(o):return o['inputs'][:1] if o['cid']==0x8f else o['inputs']
computable=[]
for o in document['ops']:
    computable.append(o['cid'] in ids and o['anim']==b'\1' and all(computable[i] for i in geometry_inputs(o)) and (o['cid']!=0x8f or o['inputs'][1] in pixel_ids))
roots=sorted({i for o,c in zip(document['ops'],computable) if not c for i in o['inputs']+[x for x in o['links'] if x is not None] if computable[i]})
needed=set(roots)
for i in reversed(range(len(document['ops']))):
    if i in needed:needed.update(geometry_inputs(document['ops'][i]))
words=[len(document['ops'])]
for i,o in enumerate(document['ops']):
    pack=document['classes'][o['cls']][2].lower()
    params=[]
    for fmt,v in zip(pack,o['par']):
        params.append(0 if fmt=='-' else struct.unpack('<I',struct.pack('<f',v))[0] if fmt in 'gfe' else int(v,16) if isinstance(v,str) else v&0xffffffff)
    words.extend([o['cid'],int(i in needed),len(o['inputs']),len(o['links']),len(params)])
    words.extend(o['inputs']);words.extend(0xffffffff if x is None else x for x in o['links']);words.extend(params)
words.extend([len(roots),*roots])
input_data=struct.pack('<'+'I'*len(words),*words)

support=deform.adapt_support(assembly.support.replace("struct GenMesh{","struct GenBitmap;struct GenMesh{"))
support=support.replace('#define sPLAYER 1','#define sPLAYER 1\n#define sINTRO 1\n#define KC_MATERIAL 3\n#define SCRIPTVERIFY(x) assert(x)')
support=support.replace('struct DummyMaterial{int Id;', 'struct DummyMaterial{int Id;int ClassId=3;')
support=support.replace('struct GenMeshMtrl{','using GenMaterial=DummyMaterial;using sF323=Triple;\nstruct GenMeshMtrl{')
support=support.replace('bool CheckMesh(GenMesh*,int=0){return false;}', 'bool CheckMesh(GenMesh*m,int mask=0){if(!m)return true;if(!(uint32_t(mask)&0x80000000)){if(mask)m->Mask2Sel(mask);else m->All2Sel();}return false;}')
support=support.replace('void Init(float a,float b,float c,float d)', '''
 void Init(){Init(0,0,0,1);}
 void Mul4(const sVector&a){x*=a.x;y*=a.y;z*=a.z;w*=a.w;}
 void Add4(const sVector&a,const sVector&b){x=a.x+b.x;y=a.y+b.y;z=a.z+b.z;w=a.w+b.w;}
 void Scale4(float f){x*=f;y*=f;z*=f;w*=f;}
 void Init(float a,float b,float c,float d)''')
support=support.replace('void Mask2Sel(sU32);','''int SkipFaceEdge(sU32,int);void Subdivide(float);void Triangulate(int,sU32,int,int=0);void Edge2Vert(int);void DeleteFaces();void Mask2Sel(sU32);''')
support=support.replace('struct GenBitmap{uint64_t*Data;int XSize,YSize;};','struct GenBitmap{uint64_t*Data;int XSize,YSize;void Release(){}};')
support+='''
#include <fstream>
void sSetMem(void*p,int value,size_t n){memset(p,value,n);}
GenMesh*Mesh_DeleteFaces(GenMesh*,int);
'''
extra=['sInt GenMesh::SkipFaceEdge(', 'void GenMesh::ReIndex(', 'static sInt MakeSubdMask(', 'void GenMesh::Subdivide(', 'void GenMesh::Triangulate(', 'void GenMesh::ExtrudeNormal(', 'void GenMesh::Edge2Vert(', 'void GenMesh::DeleteFaces(', 'void GenMesh::NeedAllNormals(', 'GenMesh * __stdcall Mesh_DeleteFaces(', 'GenMesh * __stdcall Mesh_Cylinder(', 'GenMesh * __stdcall Mesh_SelectAll(', 'GenMesh * __stdcall Mesh_SelectCube(', 'GenMesh * __stdcall Mesh_Subdivide(', 'GenMesh * __stdcall Mesh_Transform(', 'GenMesh * __stdcall Mesh_TransformEx(', 'GenMesh * __stdcall Mesh_Triangulate(', 'GenMesh * __stdcall Mesh_ShadowEnable(', 'GenMesh * __stdcall Mesh_SingleVert(', 'void GenMeshElem::SelLogic(', 'static void MultiSplit(', 'static void QuadSplit(', 'GenMesh * __stdcall Mesh_Grid(', 'GenMesh * __stdcall Mesh_SelectLogic(', 'GenMesh * __stdcall Mesh_Extrude(', 'GenMesh * __stdcall Mesh_Color(', 'GenMesh * __stdcall Mesh_LightSlot(', 'void GenMesh::CalcNormal(', 'void GenMesh::CalcFaceNormal(', 'void GenMesh::CubicProjection(', 'GenMesh * __stdcall Mesh_UVProjection(', 'void GenMesh::CalcBBox(', 'GenMesh * __stdcall Mesh_Center(', 'GenMesh * __stdcall Mesh_AutoCollision(', 'void GenMesh::Displace(', 'GenMesh * __stdcall Mesh_Displace(', 'void GenMesh::Bevel(', 'GenMesh * __stdcall Mesh_Bevel(', 'GenMesh * __stdcall Mesh_Bend(', 'GenMesh * __stdcall Mesh_Bend2(']
functions=assembly.functions+'\n'+'\n'.join(oracle.function(s) for s in extra)
old_source=oracle.source
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genmaterial.cpp').read_text(encoding='cp1252')
functions+='\n'+oracle.function('GenMesh * __stdcall Mesh_MatLink(')
oracle.source=cube.types
functions+='\n'+'\n'.join(oracle.function(s) for s in ['void sMatrix::InitEuler(', 'void sMatrix::Trans3(', 'void sMatrix::TransR('])
oracle.source=old_source
support=support.replace('storage(131072)','storage(5120)')
support=support.replace('void Realloc(int){}','void Realloc(int n){if(n*5>(int)storage.size()){storage.resize(std::max(n*5,(int)storage.size()*3/2));VertBuf=storage.data();}}')
support=support.replace('Vert.AtLeast(Vert.Count);return i;','Vert.AtLeast(Vert.Count);Realloc(Vert.Count);return i;')
support=support.replace('void Init(int,int){}','void Init(int,int n){Realloc(n);}')
support=support.replace('void InitEuler(float,float,float);','const sVector&operator[](int n)const{return (&i)[n];}void MulA(const sMatrix&,const sMatrix&);void MulA(const sMatrix&b){MulA(*this,b);}void InitRandomSRT(const float*);void InitEuler(float,float,float);')
support=support.replace('struct sMatrix{','struct sAABox;struct sMatrix{')
support=support.replace('void Mask2Sel(sU32);','void CalcBBox(sAABox&);void Mask2Sel(sU32);')
support+='\nstruct sAABox{sVector Min,Max;};\n'
support=support.replace('void Init(float a,float b,float c,float d)','void Mul3(const sVector&a){x*=a.x;y*=a.y;z*=a.z;}void InitColor(uint32_t c){x=((c>>16)&255)/255.0f;y=((c>>8)&255)/255.0f;z=(c&255)/255.0f;w=((c>>24)&255)/255.0f;}void Init(float a,float b,float c,float d)')
support+='\nfloat sFInvSqrt(float f){return 1.0f/sqrtf(f); }\n'
support=support.replace('void Mask2Sel(sU32);','void CalcNormal(sVector&,int,int,int);void CalcFaceNormal(sVector&,int);void CubicProjection(const sMatrix&,int,int,bool);void Mask2Sel(sU32);')
support+='\nfloat sFATan2(float a,float b){return atan2f(a,b);}float sFSqrt(float f){return sqrtf(f);}const float sPIF=3.14159265358979323846f;\n'
support=support.replace('struct Triple{float x,y,z;};','struct Triple{float x,y,z;float&operator[](int i){return (&x)[i];}};')
support+='\ndouble sFPow(double a,double b){return pow(a,b); }\n'
support=support.replace('void SelElem(sU32,sBool,sInt);','void SelElem(sU32,sBool,sInt);void SelLogic(sU32,sU32,sU32,int);')
support=support.replace('void CalcNormals(int,int);','void CalcNormals(int=0,int=3);')
support=support.replace('void SplitBridge(sU32,sU32,int&,int&,sU32,int,int* =nullptr,int* =nullptr);','void SplitBridge(sU32,sU32,int&,int&,sU32=0,int=0,int* =nullptr,int* =nullptr);')
support=support.replace('void Init(float a,float b,float c,float d)','void Lin4(const sVector&a,const sVector&b,float t){x=a.x+(b.x-a.x)*t;y=a.y+(b.y-a.y)*t;z=a.z+(b.z-a.z)*t;w=a.w+(b.w-a.w)*t;}void Init(float a,float b,float c,float d)')
support=support.replace('sVector&VertPos(int i)','sVector&VertNorm(int i){return VertBuf[i*5+1];}sVector&VertPos(int i)')
support+='\nGenMesh*Mesh_SelectLogic(GenMesh*,sU32,sU32,sU32,int);\n'
support=perlin3.support(support)
support=support.replace('struct sVector{float&operator[](int i){return (&x)[i];}', 'struct sVector{float&operator[](int i){switch(i){case 0:return x;case 1:return y;case 2:return z;default:return w;}}')
support=support.replace('struct Triple{float x,y,z;float&operator[](int i){return (&x)[i];}};', 'struct Triple{float x,y,z;float&operator[](int i){switch(i){case 0:return x;case 1:return y;default:return z;}}};')
# (&t.x)[i] crosses separate scalar members. Modern Clang -O2 eliminates the
# Grid z-edge test; -O0 and the intended MSVC contiguous-vector layout agree.
functions=functions.replace('(&t.x)[i]','t[i]')
functions+=perlin3.functions
oracle.source=cube.types
functions+='\n'+oracle.function('sF32 sFGetRnd()')+'\n'+oracle.function('void sMatrix::InitRandomSRT(')+'\n'+oracle.function('void sMatrix::MulA(')
oracle.source=old_source
functions+='\n'+oracle.function('GenMesh * __stdcall Mesh_Multiply(')
writer=assembly.writer+'\nword(m.GotNormals);'
driver=r'''
template<class T>void fix(Storage<T>&a){a.Array=a.data.data();}
GenMesh*copy(GenMesh*s){auto*m=new GenMesh(*s);m->VertBuf=m->storage.data();fix(m->Vert);fix(m->Edge);fix(m->Face);fix(m->Mtrl);fix(m->Coll);fix(m->Lgts);fix(m->Parts);return m;}
struct Image{int width=0,height=0;std::vector<uint64_t>data;};
struct Op{int id;bool needed;std::vector<uint32_t>inputs,links,p;int users=0;GenMesh*cache=nullptr;};
int main(int argc,char**argv){assert(argc==5);verify_perlin3(argv[4]);verify_bilinear(argv[3]);std::ifstream input(argv[1],std::ios::binary);std::vector<Op>ops(read(input));
for(auto&o:ops){o.id=read(input);o.needed=read(input);int ni=read(input),nl=read(input),np=read(input);for(int j=0;j<ni;j++)o.inputs.push_back(read(input));for(int j=0;j<nl;j++)o.links.push_back(read(input));for(int j=0;j<np;j++)o.p.push_back(read(input));o.p.resize(32);}
std::vector<int>roots(read(input));for(auto&i:roots){i=read(input);ops[i].users++;}
for(auto&o:ops)if(o.needed)for(unsigned j=0;j<o.inputs.size();j++)if(o.id!=0x8f||j==0)ops[o.inputs[j]].users++;
std::vector<Image>images(ops.size());std::ifstream pixels(argv[2],std::ios::binary);int nimages=read(pixels);
while(nimages--){int id=read(pixels);auto&im=images[id];im.width=read(pixels);im.height=read(pixels);read(pixels);read(pixels);read(pixels);im.data.resize(im.width*im.height);pixels.read((char*)im.data.data(),im.data.size()*8);assert(pixels);}

std::vector<KOp>events(ops.size());std::vector<DummyMaterial>materials(ops.size());for(int i=0;i<(int)ops.size();i++){events[i].Id=i;materials[i].Id=i+2;}
for(auto&o:ops){if(!o.needed)continue;auto&p=o.p;auto f=[&](int n){float v;memcpy(&v,&p[n],4);return v;};sFSRT srt;
GenMesh*m=o.inputs.empty()?nullptr:copy(ops[o.inputs[0]].cache);auto event=[&](int n)->KOp*{return n<(int)o.links.size()&&o.links[n]!=0xffffffff?&events[o.links[n]]:nullptr;};
switch(o.id){
case 0x81:memcpy(srt.v,p.data()+4,36);m=Mesh_Cube(p[0],p[1],p[2],p[3],srt);break;
case 0x82:m=Mesh_Cylinder(p[0],p[1],p[2],p[3],p[4]);break;
case 0x85:m=Mesh_SelectAll(m,p[0],p[1]);break;
case 0x86:m=Mesh_SelectCube(m,p[0],p[1],{f(2),f(3),f(4)},{f(5),f(6),f(7)});break;
case 0x87:m=Mesh_Subdivide(nullptr,m,p[0],f(1),p[2]);break;
case 0x88:memcpy(srt.v,p.data()+1,36);m=Mesh_Transform(nullptr,m,p[0],srt);break;
case 0x89:memcpy(srt.v,p.data()+1,36);m=Mesh_TransformEx(nullptr,m,p[0],srt,p[10],p[11]);break;
case 0x8c:m=Mesh_Triangulate(m,p[0],p[1],p[2]);break;
case 0x8f:{auto&image=images[o.inputs[1]];assert(!image.data.empty());GenBitmap bitmap{image.data.data(),image.width,image.height};m=Mesh_Displace(nullptr,m,&bitmap,p[0],{f(1),f(2),f(3)});break;}
case 0x91:CheckMesh(m,p[0]<<16);memcpy(srt.v,p.data()+1,36);{sMatrix mat;mat.InitSRT(srt.v);m->Perlin(mat,{f(10),f(11),f(12),0});}break;
case 0x90:m=Mesh_Bevel(nullptr,m,p[0],f(1),f(2),p[3]);break;
case 0xa5:memcpy(srt.v,p.data()+1,36);m=Mesh_UVProjection(m,p[0],srt,p[10]);break;
case 0xa6:m=Mesh_Center(m,p[0],p[1]);break;
case 0xa7:m=Mesh_AutoCollision(m,f(0),p[1],p[2],p[3],p[4],p[5]);break;
case 0xa0: {sFSRT other;memcpy(srt.v,p.data()+1,36);memcpy(other.v,p.data()+10,36);m=Mesh_Bend(nullptr,m,p[0],srt,other,{f(19),f(20)},{f(21),f(22)},p[23]);break;}
case 0xab:m=Mesh_Bend2(m,{f(0),f(1),f(2)},{f(3),f(4),f(5)},f(6),f(7));break;
case 0x92:CheckMesh(m);for(unsigned j=1;j<o.inputs.size();j++)m->Add(ops[o.inputs[j]].cache);m->GotNormals=false;break;
case 0x93:m=Mesh_DeleteFaces(m,p[0]);break;
case 0x95:memcpy(srt.v,p.data(),36);m=Mesh_Multiply(nullptr,m,srt,p[9],p[10],f(11),f(12),{f(13),f(14),f(15)},f(16));break;
case 0x96:assert(!o.links.empty()&&o.links[0]!=0xffffffff);m=Mesh_MatLink(m,&materials[o.links[0]],p[0],p[1]);break;
case 0x9a:m=Mesh_Extrude(nullptr,m,p[0],p[1],p[2],p[3],{f(4),f(5),f(6)},{f(7),f(8),f(9)},{f(10),f(11),f(12)});break;
case 0x9d:m=Mesh_Grid(p[0],p[1],p[2]);break;
case 0x9c:m=Mesh_CollisionCube(m,event(0),event(1),f(0),f(1),f(2),f(3),f(4),f(5),p[6],p[7],p[8],p[9],p[10],p[11],p[12],p[13]);break;
case 0xad:m=Mesh_Color(m,{f(0),f(1),f(2)},{f(3),f(4)},p[5],f(6),f(7),p[8]);break;
case 0xaf:m=Mesh_LightSlot({f(0),f(1),f(2)},p[3],f(4),f(5));break;
case 0xb0:m=Mesh_ShadowEnable(m,p[0]);break;
case 0xb3:m=Mesh_SingleVert();break;
default:assert(false);}
assert(m);o.cache=m;
for(unsigned j=0;j<o.inputs.size();j++)if(o.id!=0x8f||j==0){auto i=o.inputs[j];if(--ops[i].users==0){delete ops[i].cache;ops[i].cache=nullptr;}}
}
word(roots.size());for(int id:roots){word(id);auto&m=*ops[id].cache;
'''+writer+r'''
}for(auto&o:ops)delete o.cache;
}
'''
if "--grid-layout-check" in sys.argv:
    layout_driver='int main(){auto*p=Mesh_Grid(2,92,92);auto&m=*p;'+writer+'delete p;}'
    results={}
    with tempfile.TemporaryDirectory() as tmp:
        for label,optimization,body in [('legacy_o0','-O0',functions.replace('sFAbs(t[i])','sFAbs((&t.x)[i])')),('explicit_o2','-O2',functions)]:
            cpp=pathlib.Path(tmp)/(label+'.cpp');exe=pathlib.Path(tmp)/label
            cpp.write_text(perlin3.prefix+support+body+cube.cube+layout_driver)
            subprocess.run(['clang++','-std=c++17','-Wno-register',optimization,'-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
            result=subprocess.check_output([str(exe)])
            results[label]={'bytes':len(result),'sha256':hashlib.sha256(result).hexdigest()}
    assert results['legacy_o0']==results['explicit_o2'],results
    (root/'.reference/grid-layout-comparison.json').write_text(json.dumps(results,indent=2)+'\n')
    print(f"Grid legacy/O0 and explicit/O2 match: {results['legacy_o0']}")
    raise SystemExit(0)

special="--multiply-cases" in sys.argv
if special:
    driver=r'''
int main(){word(64);for(int c=0;c<64;c++){
 sFSRT base={{{1,1,1},{0,0,0},{0,0,0}}};auto*input=Mesh_Cube(1+c%2,1+(c/2)%2,1,(c%16)|((c/16)%4)*16,base);
 sFSRT step={{{1.03125f,.875f,1.125f},{.0625f,-.03125f,.125f},{0,.25f,-.125f}}};
 auto*p=Mesh_Multiply(nullptr,input,step,c%7-1,c%4,-.125f,.25f,{.015625f*(c%3),.03125f,-.015625f},(c&4)?.0625f:0.f);auto&m=*p;
'''+writer+r'''
 delete p;
}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp,exe,inp=pathlib.Path(tmp)/'graph.cpp',pathlib.Path(tmp)/'graph',pathlib.Path(tmp)/'input.bin'
    cpp.write_text(perlin3.prefix+support+functions+cube.cube+perlin3.gate+driver);inp.write_bytes(input_data)
    subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    output=subprocess.check_output([str(exe),str(inp),str(pixel_path),str(root/"tests/fixtures/bitmap-bilinear.bin"),str(root/"tests/fixtures/mesh-perlin3.bin")])
out=root/('tests/fixtures/mesh-multiply.bin' if special else '.reference/mesh-graph-beta.bin' if beta else 'tests/fixtures/mesh-graph3383.wz')
out.write_bytes(output if beta or special else pack_fixture(output))
print(f'{out}: '+('64 original multiplication cases' if special else f'{len(roots)} composed geometry outputs / {len(needed)} original operators')+f' / {len(output)} bytes')
