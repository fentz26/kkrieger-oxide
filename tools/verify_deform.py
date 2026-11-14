#!/usr/bin/env python3
"""Original Bend/Bend2/Bevel/Displace pipelines, with a verified bilinear adapter."""
import pathlib
import subprocess
import tempfile
import verify_cube as cube
import verify_mesh as oracle
from oracle_mm import MMX

root=oracle.root
def adapt_support(base):
    support=base.replace('#define sPLAYER 1','#define sPLAYER 1\n#define sINTRO 1\n#define sGMF_UV1 64\n#define sGMF_UV2 128\n#define sGMF_UV3 256\n#define sGMI_UV1 6\n#define sGMI_UV2 7\n#define sGMI_UV3 8\n#define sSwap std::swap')
    support=support.replace('#include <vector>','#include <vector>\n#include <algorithm>\n#include <fstream>\nusing sU16=uint16_t;using sU64=uint64_t;')
    # Aliases follow stdint, not before it.
    support=support.replace('using sU16=uint16_t;using sU64=uint64_t;','').replace('using sInt=int;', 'using sU16=uint16_t;using sU64=uint64_t;using sInt=int;')
    support=support.replace('struct Triple{','struct Pair{float x,y;};using sF322=Pair;\nstruct Triple{')
    support=support.replace('struct KOp{};', 'using sF323=Triple;struct GenBitmap;struct KOp{};')
    support=support.replace('struct sVector{', 'struct sMatrix;\nstruct sVector{')
    support=support.replace('void Init(float a,float b,float c,float d)', '''
     void Rotate3(const sMatrix&);void Rotate34(const sMatrix&,const sVector&);
     void Neg3(){x=-x;y=-y;z=-z;}
     void Lin3(const sVector&a,const sVector&b,float t){x=a.x+(b.x-a.x)*t;y=a.y+(b.y-a.y)*t;z=a.z+(b.z-a.z)*t;}
     void Add4(const sVector&a){x+=a.x;y+=a.y;z+=a.z;w+=a.w;}
     void AddScale4(const sVector&a,float f){x+=a.x*f;y+=a.y*f;z+=a.z*f;w+=a.w*f;}
     void Scale4(const sVector&a,float f){x=a.x*f;y=a.y*f;z=a.z*f;w=a.w*f;}
     float Abs3(){return sqrtf(Dot3(*this));}
     void Add3(const sVector&a,const sVector&b){x=a.x+b.x;y=a.y+b.y;z=a.z+b.z;}
     void Init(float a,float b,float c,float d)''')
    support=support.replace('void InitEulerPI2(const float*);','void InitEuler(float,float,float);void Trans3();void TransR();void InitEulerPI2(const float*);')
    support=support.replace('Storage<GenMeshFace>Face;', 'Storage<GenMeshFace>Face;bool GotNormals=false;')
    support=support.replace('void Mask2Sel(sU32);','void ReIndex();void Bevel(float,float,int,sU32,int);void NeedAllNormals();void ExtrudeNormal(float);void Displace(GenBitmap*,float,float,float);void Mask2Sel(sU32);')
    support=support.replace('bool CheckMesh(GenMesh*,int=0){return false;}', 'bool CheckMesh(GenMesh*m,int mask=0){if(!m)return true;if(!(uint32_t(mask)&0x80000000)){if(mask)m->Mask2Sel(mask);else m->All2Sel();}return false;}')
    support+='''
    void sVector::Rotate34(const sMatrix&m,const sVector&v){Scale4(m.i,v.x);AddScale4(m.j,v.y);AddScale4(m.k,v.z);Add4(m.l);}
    void sVector::Rotate3(const sMatrix&m){auto v=*this;Scale3(m.i,v.x);AddScale3(m.j,v.y);AddScale3(m.k,v.z);}
    template<class T>T sRange(T v,T high,T low){return v<low?low:v>high?high:v;}
    template<class T>T sMin(T a,T b){return a<b?a:b;}template<class T>T sMax(T a,T b){return a>b?a:b;}
    int sGetPower2(int n){int i=0;while((1<<i)<n)i++;return i;}
    uint16_t lane16(uint64_t a,int i){return a>>(i*16);}
    uint16_t twice(uint16_t a,uint16_t b){return uint16_t(int32_t(int16_t(a))*int16_t(b)>>16)<<1;}
    uint16_t usub(uint16_t a,uint16_t b){return a>b?a-b:0;}
    '''+MMX[MMX.index('struct BilinearContext'):]+'''
    struct GenBitmap{uint64_t*Data;int XSize,YSize;};
    uint32_t read(std::ifstream&in){uint32_t v;in.read((char*)&v,4);assert(in);return v;}
    void verify_bilinear(const char*path){std::ifstream in(path,std::ios::binary);auto n=read(in),w=read(in),h=read(in);std::vector<uint64_t>image(w*h);in.read((char*)image.data(),image.size()*8);while(n--){int u=read(in),v=read(in),border=read(in);uint64_t expected=read(in);expected|=uint64_t(read(in))<<32;BilinearContext c;BilinearSetup(&c,image.data(),w,h,border);uint64_t actual;BilinearFilter(&c,&actual,u,v);assert(actual==expected);}}
    '''
    return support

support=adapt_support(cube.support)
functions=cube.functions+'\n'+'\n'.join(oracle.function(s) for s in ['void GenMesh::ReIndex(', 'void GenMesh::NeedAllNormals(', 'void GenMesh::ExtrudeNormal(', 'void GenMesh::Bevel(', 'void GenMesh::Displace(', 'GenMesh * __stdcall Mesh_Bend(', 'GenMesh * __stdcall Mesh_Bend2('])
old_source=oracle.source
oracle.source=cube.types
functions+='\n'+'\n'.join(oracle.function(s) for s in ['void sMatrix::InitEuler(', 'void sMatrix::Trans3(', 'void sMatrix::TransR('])
oracle.source=old_source
driver='''
int main(int argc,char**argv){assert(argc==2);verify_bilinear(argv[1]);word(64);
std::vector<uint16_t>pixels(32*16*4);for(int y=0;y<16;y++)for(int x=0;x<32;x++)for(int j=0;j<4;j++)pixels[(y*32+x)*4+j]=(x*113+y*349+j*997)%32768;
GenBitmap bitmap{(uint64_t*)pixels.data(),32,16};
for(int c=0;c<64;c++){
 sFSRT base={{{1,1,1},{0,0,0},{0,0,0}}};auto*p=Mesh_Cube(1+c%2,2+(c/2)%2,1,c%2,base);auto&m=*p;
 for(int i=0;i<m.Vert.Count;i++)m.Vert[i].Mask=i%3?1:0;
 sFSRT a={{{1+(c%3)*0.125f,1,.75f},{.015625f,-.03125f,0},{0,-.125f,.25f}}};
 sFSRT b={{{.5f,1.25f,1},{-.0625f,0,.03125f},{.25f,0,-.125f}}};
 Mesh_Bend(nullptr,p,c%2,a,b,{.0625f,-.03125f},{-.75f,.625f},c%8);
 Mesh_Bend2(p,{.125f,-.25f,.0625f},{.03125f,-.0625f,.015625f},.25f+(c%3)*.25f,(c%7-3)*.0625f);
 m.NeedAllNormals();m.All2Sel(false);m.Face[0].Select=true;m.Bevel((c%3)*.05f,(c%4-1)*.03125f,c%2,0,0);
 m.Displace(&bitmap,.125f,-.25f,.375f);
'''+cube.writer+'\nword(m.GotNormals);delete p;}}'
def main():
    with tempfile.TemporaryDirectory() as tmp:
        cpp,exe=pathlib.Path(tmp)/'deform.cpp',pathlib.Path(tmp)/'deform'
        cpp.write_text(support+functions+cube.cube+driver)
        subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
        data=subprocess.check_output([str(exe),str(root/'tests/fixtures/bitmap-bilinear.bin')])
    out=root/'tests/fixtures/mesh-deform.bin'
    out.write_bytes(data)
    print(f'{out}: 64 original Bend/Bend2/Bevel/Displace pipelines')

if __name__=='__main__':
    main()
