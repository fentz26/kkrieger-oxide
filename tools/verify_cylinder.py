#!/usr/bin/env python3
"""Unchanged original Cylinder, triangulation and cap/UV construction."""
import pathlib
import subprocess
import tempfile
import sys
import verify_cube as cube
import verify_mesh as oracle

root=oracle.root
support=cube.support.replace('void Init(float a,float b,float c,float d)', 'void Init(){Init(0,0,0,1);}\n void Mul4(const sVector&v){x*=v.x;y*=v.y;z*=v.z;w*=v.w;}\n void AddScale4(const sVector&v,float f){x+=v.x*f;y+=v.y*f;z+=v.z*f;w+=v.w*f;}\n void Init(float a,float b,float c,float d)')
support=support.replace('void CalcNormals(int,int);','void Triangulate(int,sU32,int,int);void ExtrudeNormal(float);void Edge2Vert(int);void DeleteFaces();void CalcNormals(int,int);')
support=support.replace('bool CheckMesh(GenMesh*,int=0){return false;}', 'bool CheckMesh(GenMesh*m,int mask=0){if(!m)return true;if(!(uint32_t(mask)&0x80000000)){if(mask)m->Mask2Sel(mask);else m->All2Sel();}return false;}')
support+='''\nvoid sSetMem(void*p,int value,size_t n){memset(p,value,n);}
template<class T>T sRange(T value,T high,T low){return value<low?low:value>high?high:value;}
GenMesh*Mesh_DeleteFaces(GenMesh*,int);
'''
functions=cube.functions+'\n'+'\n'.join(oracle.function(s) for s in ['void GenMesh::Triangulate(', 'void GenMesh::ExtrudeNormal(', 'void GenMesh::Edge2Vert(', 'void GenMesh::DeleteFaces(', 'GenMesh * __stdcall Mesh_DeleteFaces(', 'GenMesh * __stdcall Mesh_Cylinder('])
parameters=[(3+c%13,1+(c//13)%4,c%4,1+(c//52)%3,(c//4)%(3+c%13)) for c in range(192)]
suffix=''
if '--development' in sys.argv or '--beta' in sys.argv:
    sys.path.insert(0,str(root/'.reference/wasm'))
    import kxread
    beta='--beta' in sys.argv
    doc=kxread.read((root/('.reference/verified-beta.kx' if beta else 'assets/kkrieger3383.kx')).read_bytes(),beta=beta)
    parameters=[o['par'] for o in doc['ops'] if o['cid']==0x82]
    suffix='-beta' if beta else '3383'
table='const int cases[][5]={'+','.join('{'+','.join(str(v) for v in p)+'}' for p in parameters)+'};'
driver=table+f'\nint main(){{word({len(parameters)});for(auto&params:cases){{for(int v:params)word(v);auto*p=Mesh_Cylinder(params[0],params[1],params[2],params[3],params[4]);auto&m=*p;\n'+cube.writer+'\nword(m.Parts.Count);for(int i=0;i<m.Parts.Count;i++)word(m.Parts[i]);delete p;}}'
with tempfile.TemporaryDirectory() as tmp:
    cpp,exe=pathlib.Path(tmp)/'cylinder.cpp',pathlib.Path(tmp)/'cylinder'
    cpp.write_text(support+functions+driver)
    subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/(('.reference/' if suffix=='-beta' else 'tests/fixtures/')+'mesh-cylinder'+suffix+'.bin')
out.write_bytes(data)
print(f'{out}: {len(parameters)} original cylinders')
