#!/usr/bin/env python3
"""Full-resolution original C++ texture graph checks, excluding font-dependent roots.

Font-dependent outputs are explicitly omitted pending a Windows GDI oracle.
All remaining original graph branches execute unchanged operator functions;
low-level scalar wrappers are checked against original MMX fixtures first.
"""
import os
import pathlib
import struct
import subprocess
import sys
import verify_generators as oracle

root, ref, data = oracle.root, oracle.ref, oracle.data
def is_texture(cid):
    return 0x21 <= cid <= 0x39
computable = []
for op in data["ops"]:
    computable.append(is_texture(op["cid"]) and op["cid"] != 0x31
                      and all(computable[i] for i in op["inputs"]))
roots = sorted({i for op in data["ops"] if not is_texture(op["cid"])
                for i in op["inputs"] + [i for i in op["links"] if i is not None]
                if computable[i]})
raw_displacement="--displacement-inputs" in sys.argv
if raw_displacement:
    roots=sorted({op['inputs'][1] for op in data['ops'] if op['cid']==0x8f and len(op['inputs'])==2 and computable[op['inputs'][1]]})
out = bytearray(struct.pack("<I", len(data["ops"])))
for op in data["ops"]:
    pack = data["classes"][op["cls"]][2].lower()
    count = max(op["conv"] & 255, len(pack))
    out.extend(struct.pack("<III", op["cid"], len(op["inputs"]), count))
    for i in op["inputs"]:
        out.extend(struct.pack("<I", i))
    for i in range(count):
        if i >= len(pack) or pack[i] == "-":
            out.extend(b"\0" * 4)
        elif pack[i] in "gfe":
            out.extend(struct.pack("<f", op["par"][i]))
        else:
            v = op["par"][i]
            out.extend(struct.pack("<I", int(v,16) if isinstance(v,str) else v & 0xFFFFFFFF))
out.extend(struct.pack("<I", len(roots)))
for i in roots:
    out.extend(struct.pack("<I", i))
input_path = ref / "texture-graph-input.bin"
input_path.write_bytes(out)
flat = oracle.extract(oracle.source,"GenBitmap * __stdcall Bitmap_Flat(","GenBitmap * __stdcall Bitmap_RenderTarget(")
mask = oracle.extract(oracle.source,"GenBitmap * __stdcall Bitmap_Mask(","GenBitmap * __stdcall Bitmap_Rotate(")
extra = r'''
void sSetMem8(uint64_t*p,uint64_t value,int count){std::fill(p,p+count,value);}
'''
# Retain the original primitive verification gate from the scalar driver.
primitive_check = oracle.driver[:oracle.driver.index("int main(")]
driver = r'''
struct Op{int id;std::vector<uint32_t> inputs,p;bool needed=false;int users=0;GenBitmap*cache=nullptr;};
int main(int argc,char**argv){
 if(argc!=3)return 2;verify_primitives(argv[2]);sInitPerlin();std::ifstream in(argv[1],std::ios::binary);
 std::vector<Op> ops(read(in));
 for(auto&o:ops){o.id=read(in);auto count=read(in),words=read(in);for(unsigned i=0;i<count;i++)o.inputs.push_back(read(in));for(unsigned i=0;i<words;i++)o.p.push_back(read(in));}
 std::vector<int> roots(read(in)),pending;
 for(auto&i:roots){i=read(in);pending.push_back(i);ops[i].users++;}
 while(!pending.empty()){int i=pending.back();pending.pop_back();auto&o=ops[i];if(o.needed)continue;o.needed=true;for(auto j:o.inputs)pending.push_back(j);}
 for(auto&o:ops)if(o.needed)for(auto i:o.inputs)ops[i].users++;
 for(int i=0;i<(int)ops.size();i++){
  auto&o=ops[i];if(!o.needed)continue;auto&p=o.p;
  auto f=[&](int n){float v;memcpy(&v,&p[n],4);return v;};
  std::vector<GenBitmap*> args;for(auto input:o.inputs){auto*b=ops[input].cache;assert(b);b->AddRef();args.push_back(b);}
  auto*a=args.empty()?nullptr:args[0];auto*b=args.size()>1?args[1]:nullptr;GenBitmap*r=nullptr;
  switch(o.id){
   case 0x21:r=Bitmap_Flat(p[0],p[1],p[2]);break;
   case 0x22:r=Bitmap_Perlin(p[0],p[1],p[2],p[3],f(4),p[5],p[6],f(7),f(8),p[9],p[10]);break;
   case 0x23:r=Bitmap_Color(a,p[0],p[1]);break;
   case 0x24:{
    if(args.size()==1){CheckBitmap(a);r=a;break;}
    int mode=p[0];if(mode==5)mode=12;else if(mode>=6&&mode<=8)mode+=10;
    GenBitmap*base;CheckBitmap(a,&base);
    for(unsigned j=1;j<args.size();j++){assert(a->Size==args[j]->Size);Bitmap_Inner(a->Data,args[j]->Data,a->Size,mode,base->Data);args[j]->Release();base=a;}
    r=a;break;
   }
   case 0x25:r=Bitmap_Format(a,p[0],p[1],p[2]);break;
   case 0x27:case 0x39:r=Bitmap_GlowRect(a,f(0),f(1),f(2),f(3),f(4),f(5),p[6],f(7),f(8),p[9],p[10]);break;
   case 0x29:r=Bitmap_Blur(a,p[0],f(1),f(2),f(3));break;
   case 0x2a:r=Bitmap_Mask(a,b,args.at(2),p[0]);break;
   case 0x2b:r=Bitmap_HSCB(a,f(0),f(1),f(2),f(3));break;
   case 0x2c:r=Bitmap_Rotate(a,f(0),f(1),f(2),f(3),f(4),p[5],p.size()>6?p[6]:0,p.size()>7?p[7]:0);break;
   case 0x2d:r=Bitmap_Distort(a,b,f(0),p[1]);break;
   case 0x2e:r=Bitmap_Normals(a,f(0),p[1]);break;
   case 0x30:r=Bitmap_Bump(a,b,p[0],f(1),f(2),f(3),f(4),f(5),p[6],p[7],f(8),f(9),f(10),p[11],f(12),f(13));break;
   case 0x32:r=Bitmap_Cell(p[0],p[1],p[2],p[3],p[4],p[5],p[6],f(7),f(8),p[9],f(10),p[11],f(12));break;
   case 0x34:r=Bitmap_Gradient(p[0],p[1],p[2],p[3],f(4),f(5),f(6),p[7]);break;
   case 0x35:r=Bitmap_Range(a,p[0],p[1],p[2]);break;
   case 0x36:r=Bitmap_RotateMul(a,f(0),f(1),f(2),f(3),f(4),p[5],p[6],p[7],p[8],p[9]);break;
   case 0x37:r=Bitmap_Twirl(a,f(0),f(1),f(2),f(3),f(4),f(5),p[6]);break;
   case 0x38:r=Bitmap_Sharpen(a,p[0],f(1),f(2),f(3));break;
   default:fprintf(stderr,"unsupported oracle node %d class %x\n",i,o.id);abort();
  }
  assert(r);o.cache=r;
  for(auto input:o.inputs)if(--ops[input].users==0){ops[input].cache->Release();ops[input].cache=nullptr;}
 }
 word(roots.size());for(auto i:roots){auto*b=ops[i].cache;assert(b);
  uint64_t h=0xcbf29ce484222325ull;auto*p=(uint8_t*)b->Data;for(int j=0;j<b->Size*8;j++)h=(h^p[j])*0x100000001b3ull;
  word(i);word(b->XSize);word(b->YSize);word(b->Format);word(b->TexMipCount);word(b->TexMipTresh);word(h);word(h>>32);
 }
 for(auto&o:ops)if(o.cache)o.cache->Release();
}
'''
if raw_displacement:
    driver=driver.replace("word(h);word(h>>32);", "pixels(b);")
cpp = ref / "texture-graph-oracle.cpp"
cpp.write_text(oracle.common_source() + extra + flat + mask + primitive_check + driver)
exe = ref / "texture-graph-oracle"
subprocess.run([os.environ.get("CXX","clang++"),"-std=c++17","-O2","-fwrapv",
                "-ffp-contract=off",str(cpp),"-o",str(exe)],check=True)
temporary = ref / "texture-graph-results.bin"
with temporary.open("wb") as stream:
    subprocess.run([str(exe),str(input_path),str(root/"tests/fixtures")],stdout=stream,check=True)
output = ref / "texture-graph-beta.bin" if oracle.beta else root / "tests/fixtures/texture-graph3383.bin"
if raw_displacement:
    output=ref/("displacement-pixels-beta.bin" if oracle.beta else "displacement-pixels3383.bin")
output.write_bytes(temporary.read_bytes())
print(f"{output}: {len(roots)} non-font roots; full-resolution reference pixels and format/mips")
