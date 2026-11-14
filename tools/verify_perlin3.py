#!/usr/bin/env python3
"""Original mesh Perlin in x87 single precision, rounding toward negative infinity.

The original control-word assembly and sFtol execute in bounded Unicorn. This
is an offline oracle; no original machine code or emulator enters the Rust game.
"""
import pathlib
import random
import struct
import subprocess
import verify_mesh as source
from x86_oracle import Oracle

root=source.root;ref=root/'.reference'
types=(ref/'fr_public/werkkzeug3_kkrieger/_types.cpp').read_text(encoding='cp1252')
header=(ref/'fr_public/werkkzeug3_kkrieger/_types.hpp').read_text(encoding='cp1252')
rng=types[types.index('static sU32 sRandomSeed'):types.index('sInt sMakePower2')]
init=types[types.index('static const sChar GTable'):types.index('#define ix')]
ftol=header[header.index('__forceinline sInt sFtol '):header.index('__forceinline sF32 sFRound ')]
mesh_perlin=source.function('void GenMesh::Perlin(')
support=r'''
using sInt=int;using sU32=unsigned;using sU8=unsigned char;using sF32=float;using sChar=char;
#define sMOBILE 1
void sCopyMem(void*d,const void*s,int n){auto*a=(sU8*)d;auto*b=(const sU8*)s;while(n--)*a++=*b++;}
template<class T>void sSwap(T&a,T&b){T t=a;a=b;b=t;}
struct sMatrix;
struct sVector{float x,y,z,w;
 float&operator[](int i){return (&x)[i];}
 void Scale4(const sVector&a,float f){x=a.x*f;y=a.y*f;z=a.z*f;w=a.w*f;}
 void AddScale4(const sVector&a,float f){x+=a.x*f;y+=a.y*f;z+=a.z*f;w+=a.w*f;}
 void Add4(const sVector&a){x+=a.x;y+=a.y;z+=a.z;w+=a.w;}
 void AddMul3(const sVector&a,const sVector&b){x+=a.x*b.x;y+=a.y*b.y;z+=a.z*b.z;}
 void Lin3(const sVector&a,const sVector&b,float t){x=a.x+(b.x-a.x)*t;y=a.y+(b.y-a.y)*t;z=a.z+(b.z-a.z)*t;}
 void Rotate34(const sMatrix&,const sVector&);
};
struct sMatrix{sVector i,j,k,l;};
void sVector::Rotate34(const sMatrix&m,const sVector&v){Scale4(m.i,v.x);AddScale4(m.j,v.y);AddScale4(m.k,v.z);Add4(m.l);}
struct GenMeshVert{sU8 Mask,Id,Select,Used;int Next,First,Temp,Temp2,ReIndex;sU8 Weight[4],Matrix[4];};
struct Vertices{GenMeshVert*Array;int Count;GenMeshVert&operator[](int i){return Array[i];}};
struct GenMesh{Vertices Vert;sVector*VertBuf;int VertSize(){return 5;}void Perlin(const sMatrix&,const sVector&);};
sVector sPerlinGradient3D[16];sU8 sPerlinPermute[512];sF32 sPerlinRandom[256][2];
'''
wrapper='''
extern "C" void initialize(){sInitPerlin();}
extern "C" void run(const sMatrix*matrix,const sVector*amp,GenMeshVert*vertices,int count,sVector*data){GenMesh mesh={{vertices,count},data};mesh.Perlin(*matrix,*amp);}
'''
cpp=ref/'perlin3-oracle.cpp';obj=ref/'perlin3-oracle.obj'
cpp.write_text(support+rng+init+ftol+mesh_perlin+wrapper)
subprocess.run(['clang++','-target','i686-pc-windows-msvc','-fms-extensions','-fasm-blocks','-ffreestanding','-fno-builtin','-fno-stack-protector','-fwrapv','-ffp-contract=off','-mno-sse','-mno-sse2','-O2','-Wno-return-type','-c',str(cpp),'-o',str(obj)],check=True)
oracle=Oracle(obj);oracle.call('_initialize')
rng=random.Random(0x503352)
output=bytearray(struct.pack('<I',128))
for case in range(128):
    matrix=[rng.randrange(-256,257)/128 if i%4!=3 else 0 for i in range(12)]+[rng.randrange(-768,769)/4 for _ in range(3)]+[1.]
    if case%8==0:matrix=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.]
    amplitude=[rng.randrange(-32,33)/16 for _ in range(3)]+[0.]
    count=16
    points=[];selected=[]
    boundaries=[0.,-0.,1.,-1.,.5,-.5,255.,256.,-256.,255.99998474121094,0.0000001,-0.0000001]
    for i in range(count):
        points.append([boundaries[(case+i+j)%len(boundaries)] if i<4 else rng.randrange(-65536,65537)/256 for j in range(3)]+[1.])
        selected.append((case+i)%5!=0)
    data=oracle.data;verts=data+0x1000;attributes=data+0x10000
    oracle.mu.mem_write(data,struct.pack('<20f',*matrix,*amplitude))
    vertex_bytes=b''.join(struct.pack('<4B5i8B',0,0,int(s),1,*([0]*5),*([0]*8)) for s in selected)
    point_bytes=b''.join(struct.pack('<20f',*p,*([0.]*16)) for p in points)
    oracle.mu.mem_write(verts,vertex_bytes);oracle.mu.mem_write(attributes,point_bytes)
    oracle.call('_run',data,data+64,verts,count,attributes)
    expected=bytes(oracle.mu.mem_read(attributes,len(point_bytes)))
    assert bytes(oracle.mu.mem_read(verts,len(vertex_bytes)))==vertex_bytes
    output.extend(struct.pack('<19fI',*matrix,*amplitude[:3],count))
    for i,(point,selection) in enumerate(zip(points,selected)):
        original=point_bytes[i*80:(i+1)*80];result=expected[i*80:(i+1)*80]
        assert result[12:]==original[12:]
        if not selection:assert result==original
        output.extend(struct.pack('<4fI',*point,int(selection)));output.extend(result[:16])
out=root/'tests/fixtures/mesh-perlin3.bin';out.write_bytes(output)
print(f'{out}: 2,048 original x87 Perlin vertices, single precision and directed rounding')
