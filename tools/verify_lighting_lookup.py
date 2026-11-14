#!/usr/bin/env python3
"""Execute original lookup texture generators with memory-only D3D allocations."""
import pathlib,subprocess,tempfile
import verify_cube as cube
import verify_mesh as oracle
root=oracle.root
support=cube.support
support+='''
#define DXERROR(x) x
#define D3DFMT_A8R8G8B8 0
#define D3DPOOL_MANAGED 0
using D3DCUBEMAP_FACES=int;
int sFtol(float x){return (int)floorf(x);}template<class T>T sMax(T a,T b){return std::max(a,b);}
struct D3DLOCKED_RECT{void*pBits;int Pitch;};struct D3DLOCKED_BOX{void*pBits;int SlicePitch,RowPitch;};
struct Cube{std::vector<unsigned char>data=std::vector<unsigned char>(6*64*64*4);void LockRect(int face,int,D3DLOCKED_RECT*lr,int,int){lr->pBits=data.data()+face*64*64*4;lr->Pitch=64*4;}void UnlockRect(int,int){}};
struct Volume{std::vector<unsigned char>data=std::vector<unsigned char>(32*32*32*4);void LockBox(int,D3DLOCKED_BOX*lb,int,int){lb->pBits=data.data();lb->SlicePitch=32*32*4;lb->RowPitch=32*4;}void UnlockBox(int){}};
struct Device{void CreateCubeTexture(int,int,int,int,int,Cube**out,int){*out=new Cube;}void CreateVolumeTexture(int,int,int,int,int,int,int,Volume**out,int){*out=new Volume;}};
struct sSystem_{Device*DXDev=new Device;Cube*DXNormalCube;Volume*DXAttenuationVolume;void MakeCubeNormalizer();void MakeAttenuationVolume();};
'''
old=oracle.source;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_start.cpp').read_text()
functions=cube.functions+'\n'+oracle.function('void sSystem_::MakeCubeNormalizer(')+'\n'+oracle.function('void sSystem_::MakeAttenuationVolume(')
oracle.source=old
# The Cube function is linked only to satisfy unused helper references.
driver='''int main(){sSystem_ s;s.MakeCubeNormalizer();s.MakeAttenuationVolume();auto&d=s.DXNormalCube->data;for(size_t i=0;i<d.size();i+=4){putchar(d[i+2]);putchar(d[i+1]);putchar(d[i]);putchar(d[i+3]);}auto&a=s.DXAttenuationVolume->data;fwrite(a.data(),1,a.size(),stdout);}'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'lookup.cpp';exe=pathlib.Path(tmp)/'lookup'
 cpp.write_text(support+functions+cube.cube+driver)
 subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/lighting-lookup.bin';out.write_bytes(data);print(f'{out}: {len(data)} original lookup bytes')
