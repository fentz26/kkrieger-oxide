"""Native offline Perlin adapter, gated against unchanged x87 machine-code outputs.

ARM FP32 uses the same directed operations as x87 PC=24 over the validated
coordinate range. Replace only x87 environment save/restore and FISTP's floor
conversion; retain the original mesh function and vector arithmetic.
"""
import re
import verify_mesh as oracle
import verify_cube as cube

prefix='''#include <cfenv>
#pragma STDC FENV_ACCESS ON
#define sMOBILE 1
'''

def support(base):
    base=base.replace('struct sVector{','struct sVector{float&operator[](int i){return (&x)[i];}void AddMul3(const sVector&a,const sVector&b){x+=a.x*b.x;y+=a.y*b.y;z+=a.z*b.z;}')
    base=base.replace('void Mask2Sel(sU32);','void Perlin(const sMatrix&,const sVector&);void Mask2Sel(sU32);')
    return base+'''\nusing sChar=char;sVector sPerlinGradient3D[16];sU8 sPerlinPermute[512];float sPerlinRandom[256][2];
int sFtol(float value){return (int)floorf(value);}
'''

rng=cube.types[cube.types.index('static sU32 sRandomSeed'):cube.types.index('sInt sMakePower2')]
init=cube.types[cube.types.index('static const sChar GTable'):cube.types.index('#define ix')]
perlin=oracle.function('void GenMesh::Perlin(')
blocks=list(re.finditer(r'__asm\s*\{.*?\}',perlin,re.S))
assert len(blocks)==2
perlin=perlin[:blocks[1].start()]+'std::fesetround(oldcw);'+perlin[blocks[1].end():]
perlin=perlin[:blocks[0].start()]+'oldcw=std::fegetround();std::fesetround(FE_DOWNWARD);'+perlin[blocks[0].end():]
functions=rng+init+perlin

gate=r'''
void verify_perlin3(const char*path){
 sInitPerlin();std::ifstream in(path,std::ios::binary);int cases=read(in);
 while(cases--){sMatrix matrix;in.read((char*)&matrix,64);sVector amplitude={0,0,0,0};in.read((char*)&amplitude,12);int count=read(in);GenMesh mesh;std::vector<uint32_t>expected(count*4);
 for(int i=0;i<count;i++){mesh.AddNewVert();in.read((char*)(mesh.VertBuf+i*5),16);mesh.Vert[i].Select=read(in);for(int j=0;j<4;j++)expected[i*4+j]=read(in);}
 mesh.Perlin(matrix,amplitude);for(int i=0;i<count;i++)assert(memcmp(mesh.VertBuf+i*5,expected.data()+i*4,16)==0);
 assert(std::fegetround()==FE_TONEAREST);
 }
}
'''
