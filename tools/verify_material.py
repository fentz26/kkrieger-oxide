#!/usr/bin/env python3
"""Compare Material11 CPU setup/updates with original source (no GPU fallback)."""
import pathlib, subprocess, tempfile, sys, struct
root = pathlib.Path(__file__).resolve().parent.parent
base = root/'.reference/fr_public/werkkzeug3_kkrieger'

def function(text, signature):
    start=text.index(signature); brace=text.index('{',start); depth=1; end=brace+1
    while depth:
        depth += (text[end]=='{')-(text[end]=='}'); end+=1
    return text[start:end]

source=(base/'genmaterial.cpp').read_text()
header=(base/'materials/material11.hpp').read_text()
constants=header[header.index('#define sMBF_ALPHATEST'):header.index('class sMaterial11')]
engine=(base/'engine.hpp').read_text()
constants+='\n'+engine[engine.index('#define ENGU_BASE'):engine.index('// animation types')]
support=r'''
#include <array>
#include <vector>
#include <cstring>
#include <cstdio>
#include <algorithm>
using sBool=bool;using sInt=int;using sU32=unsigned;using sF32=float;
#define __stdcall
#define sPLAYER 1
#define sINVALID -1
#define sPS_11 1
#define KC_BITMAP 2
#define KC_MATERIAL 3
#define sVERIFY(x) do { if(!(x)) __builtin_trap(); } while(0)
#define sRelease(x) (x)->Release()
#define sCopyMem memcpy
#define sMin std::min
struct KEnvironment{};
struct KObject{int ClassId=0;virtual ~KObject(){}void Release(){}void AddRef(){}};
struct GenBitmap:KObject{int Texture=-1,Format=1;void MakeTexture(int){Texture=Id;}int Id;GenBitmap(int id):Id(id){ClassId=KC_BITMAP;}};
struct sMaterial:KObject{};
struct sMaterial11:sMaterial{
 unsigned BaseFlags,SpecialFlags,LightFlags,pad1,TFlags[4],Combiner[13],pad3,pad4,AlphaCombiner;
 float TScale[4];unsigned Color[4];float SpecPower;unsigned pad5;float SRT1[9],SRT2[5];int ShaderLevel=0;
 int Tex[4];sMaterial11(){Reset();}
 void Reset(){memset(&BaseFlags,0,48*4);BaseFlags=0x300;SpecPower=8;for(int i=0;i<4;i++){TFlags[i]=2;Color[i]=~0u;Tex[i]=-1;}}
 void CopyFrom(sMaterial11*x){memcpy(&BaseFlags,&x->BaseFlags,48*4);ShaderLevel=x->ShaderLevel;memcpy(Tex,x->Tex,sizeof(Tex));}
 void SetTex(int i,int h){Tex[i]=h;}bool Compile(){return true;}
};
struct GenMaterialPass{sMaterial*Mtrl;int Usage,Program,Pass;float Size=0,Aspect=0;};
template<class T>struct Storage{std::vector<T>v;int Count=0;T*Add(){v.emplace_back();Count++;return &v.back();}T&operator[](int i){return v[i];}};
struct GenMaterial:KObject{Storage<GenMaterialPass>Passes;void*Insert=nullptr;GenMaterial(){ClassId=KC_MATERIAL;}void AddPass(sMaterial*m,int u,int prog,int pass=0,float size=1,float aspect=1){auto*p=Passes.Add();*p={m,u,prog,pass,size,aspect};}};
struct Material11Insert{};
struct Overlay{int CurrentShader=1;}overlay;Overlay*GenOverlayManager=&overlay;
struct KOp{unsigned p[64];KOp*links[8]{};KOp*input=nullptr;KObject*Cache=nullptr;
 KOp*GetLink(int i){return links[i];}KOp*GetInput(int i){return i==0?input:nullptr;}
 unsigned*GetAnimPtrU(int i){return p+i;}float*GetAnimPtrF(int i){return reinterpret_cast<float*>(p+i);}void ExecInputs(KEnvironment*){}};
GenMaterial*Material_Add(int,GenMaterial*a,GenMaterial*b){auto*r=new GenMaterial;for(auto*m:{a,b})for(auto p:m->Passes.v)*r->Passes.Add()=p;return r;}
void word(unsigned x){fwrite(&x,4,1,stdout);}void real(float x){unsigned u;memcpy(&u,&x,4);word(u);}
void output(GenMaterial*m){word(m->Passes.Count);word(m->Insert!=nullptr);for(auto&p:m->Passes.v){word(p.Usage);word(p.Program);word(p.Pass);real(p.Size);real(p.Aspect);auto*s=(sMaterial11*)p.Mtrl;auto*w=&s->BaseFlags;for(int i=0;i<48;i++)word(w[i]);for(auto t:s->Tex)word(t);}}
'''
multi=source[source.index('struct MultiPara'):source.index('GenMaterial * __stdcall Init_Material_Material')]
init=function(source,'GenMaterial * __stdcall Init_Material_Material(')
execute=function(source,'void __stdcall Exec_Material_Material(')
driver=r'''
int main(){word(256);for(unsigned c=0;c<256;c++){
 KOp op;for(int i=0;i<64;i++)op.p[i]=0;
 op.p[0]=0x302|(c&2);for(int i=4;i<8;i++)op.p[i]=0x1000+i;
 for(int i=8;i<21;i++)op.p[i]=0x10+i;op.p[23]=0xc;
 for(int i=24;i<48;i++)op.p[i]=0x3e800000+i*123;
 unsigned flags=(c&31)<<0;if(c&32)flags|=0x4300;if(c&64)flags|=0xb400;if(c&128)flags|=0x12000;
 if((c&31)==0)flags|=((c>>5)&3)<<16|((c>>4)&15)<<20;
 op.p[48]=flags;op.p[49]=c%7;op.p[51]=0x41800000;op.p[52]=0xaabbccdd;op.p[53]=0x12345678;
 for(int i=56;i<60;i++)op.p[i]=i+0x100;for(int i=60;i<64;i++)op.p[i]=0x3f000000+i*123;
 KOp links[8];for(int i=0;i<8;i++)if((c>> (i%8))&1){links[i].Cache=new GenBitmap(100+i);op.links[i]=&links[i];}
 for(auto x:op.p)word(x);for(int i=0;i<8;i++)word(op.links[i]?100+i:~0u);
 auto*m=Init_Material_Material(&op);op.Cache=m;output(m);
 for(int i=24;i<48;i++)op.p[i]^=0x00100000;op.p[51]^=0x00100000;op.p[52]^=0xff000000;
 for(int i=60;i<64;i++)op.p[i]^=0x00100000;
 for(auto x:op.p)word(x);Exec_Material_Material(&op,nullptr);output(m);
}}
'''
graph = '--development' in sys.argv or '--beta' in sys.argv
beta = '--beta' in sys.argv
if graph:
 sys.path.insert(0, str(root/'.reference/wasm'))
 import kxread
 kxread.round=lambda value, places: value
 data=kxread.read((root/('.reference/verified-beta.kx' if beta else 'assets/kkrieger3383.kx')).read_bytes(),beta=beta)
 payload=bytearray(struct.pack('<I',len(data['ops'])))
 for op in data['ops']:
  p=[0]*64
  if op['cid']==0xd0:
   packing=data['classes'][op['cls']][2].lower()
   for i,(fmt,v) in enumerate(zip(packing,op['par'])):
    if fmt=='-':continue
    p[i]=struct.unpack('<I',struct.pack('<f',v))[0] if fmt in 'gfe' else (int(v,16) if isinstance(v,str) else v&0xffffffff)
  payload+=struct.pack('<III',op['cid'],len(op['inputs']),len(op['links']))
  payload+=struct.pack('<64I',*p)
  for v in op['inputs']+op['links']:payload+=struct.pack('<I',0xffffffff if v is None else v)
 driver=r'''
unsigned read(FILE*f){unsigned n;if(fread(&n,4,1,f)!=1)__builtin_trap();return n;}
int main(int argc,char**argv){FILE*f=fopen(argv[1],"rb");unsigned count=read(f);std::vector<KOp>ops(count);std::vector<int>ids(count);std::vector<std::vector<unsigned>>inputs(count);unsigned materials=0;
for(unsigned i=0;i<count;i++){auto&o=ops[i];ids[i]=read(f);auto ni=read(f),nl=read(f);for(auto&v:o.p)v=read(f);for(unsigned j=0;j<ni;j++)inputs[i].push_back(read(f));for(unsigned j=0;j<nl;j++){auto link=read(f);if(link!=~0u)o.links[j]=&ops[link];}if(ids[i]>=0x21&&ids[i]<=0x39)o.Cache=new GenBitmap(i);if(ids[i]==0xd0||ids[i]==0xd1)materials++;}
for(unsigned i=0;i<count;i++){auto&o=ops[i];if(ids[i]==0xd0){if(!inputs[i].empty())o.input=&ops[inputs[i][0]];o.Cache=Init_Material_Material(&o);}else if(ids[i]==0xd1){auto*m=new GenMaterial;for(auto input:inputs[i])for(auto p:((GenMaterial*)ops[input].Cache)->Passes.v)*m->Passes.Add()=p;o.Cache=m;}}
word(materials);for(unsigned i=0;i<count;i++)if(ids[i]==0xd0||ids[i]==0xd1){word(i);output((GenMaterial*)ops[i].Cache);}
for(unsigned i=0;i<count;i++)if(ids[i]==0xd0||ids[i]==0xd1){if(ids[i]==0xd0)Exec_Material_Material(&ops[i],nullptr);word(i);output((GenMaterial*)ops[i].Cache);}
}'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'material.cpp';exe=pathlib.Path(tmp)/'material'
 cpp.write_text(constants+support+multi+init+execute+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-fno-strict-aliasing',str(cpp),'-o',str(exe)],check=True)
 args=[str(exe)]
 if graph:
  input_path=pathlib.Path(tmp)/'input';input_path.write_bytes(payload);args.append(str(input_path))
 result=subprocess.check_output(args)
out=root/('.reference/material-graph-beta.bin' if beta else 'tests/fixtures/material-graph3383.bin' if graph else 'tests/fixtures/material.bin')
out.write_bytes(result)
print(f'{out}: {len(result)} bytes of original material setup and execution')
