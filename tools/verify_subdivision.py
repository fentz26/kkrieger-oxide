#!/usr/bin/env python3
"""Execute original selected-face subdivision against creased and open regions."""
import pathlib
import subprocess
import tempfile
import verify_cube as cube
import verify_mesh as oracle

root=oracle.root
support=cube.support.replace('void Init(float a,float b,float c,float d)', '''
 void Add4(const sVector&a){x+=a.x;y+=a.y;z+=a.z;w+=a.w;}
 void Add4(const sVector&a,const sVector&b){x=a.x+b.x;y=a.y+b.y;z=a.z+b.z;w=a.w+b.w;}
 void AddScale4(const sVector&a,float f){x+=a.x*f;y+=a.y*f;z+=a.z*f;w+=a.w*f;}
 void Scale4(float f){x*=f;y*=f;z*=f;w*=f;}
 void Scale4(const sVector&a,float f){x=a.x*f;y=a.y*f;z=a.z*f;w=a.w*f;}
 void Init(float a,float b,float c,float d)''')
support=support.replace('void Mask2Sel(sU32);','int VertMask(){return 47;}int SkipFaceEdge(sU32,int);void ReIndex();void Subdivide(float);void Mask2Sel(sU32);')
support+='\nvoid sSetMem(void*p,int value,size_t n){memset(p,value,n);}\n'
functions=cube.functions+'\n'+'\n'.join(oracle.function(s) for s in ['sInt GenMesh::SkipFaceEdge(', 'void GenMesh::ReIndex(', 'static sInt MakeSubdMask(', 'void GenMesh::Subdivide('])
driver='''
int main(){word(48);for(int c=0;c<48;c++){
 int tx=1+c%2,ty=1+(c/2)%2,tz=1+(c/4)%2,flags=c%16;
 float alpha=(c%5)*0.375f;int count=1+c%2;
 sFSRT srt={{ {1,1,1},{0,0,0},{0,0,0} }};
 auto*p=Mesh_Cube(tx,ty,tz,flags,srt);auto&m=*p;
 word(tx);word(ty);word(tz);word(flags);real(alpha);word(count);
 for(int i=0;i<m.Face.Count;i++)m.Face[i].Select=c%3==0 || (i+c)%3!=0;
 for(int i=0;i<count;i++)m.Subdivide(alpha);
'''+cube.writer+'''\nword(m.Parts.Count);for(int i=0;i<m.Parts.Count;i++)word(m.Parts[i]);delete p;}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp,exe=pathlib.Path(tmp)/'subdivision.cpp',pathlib.Path(tmp)/'subdivision'
    cpp.write_text(support+functions+cube.cube+driver)
    subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/mesh-subdivision.bin'
out.write_bytes(data)
print(f'{out}: 48 original selected-face subdivision cases')
