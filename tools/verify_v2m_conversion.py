#!/usr/bin/env python3
"""Execute the original legacy V2M converter on both authored payloads."""
import pathlib,struct,subprocess,tempfile,re
ROOT=pathlib.Path(__file__).resolve().parents[1]
source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2.cpp').read_text(encoding='cp1252')
body=source[source.index('enum V2CTLTYPES'):source.index('static unsigned long GetV2MPatchData')]
support='''#include <cstdio>
#include <cstring>
#include <cmath>
#include <vector>
using sU8=unsigned char;using sU32=unsigned;using sInt=int;using sF32=float;
'''
driver=r'''
void word(unsigned n){fwrite(&n,4,1,stdout);}
int main(int argc,char**argv){sdInit();word(v2version);word(v2nparms);for(auto&p:v2parms)word(p.version);fwrite(v2initsnd,1,v2nparms,stdout);word(v2ngparms);for(auto&p:v2gparms)word(p.version);fwrite(v2initglobs,1,v2ngparms,stdout);word(argc-1);for(int a=1;a<argc;a++){FILE*f=fopen(argv[a],"rb");fseek(f,0,SEEK_END);int n=ftell(f);rewind(f);std::vector<unsigned char>b(n);fread(b.data(),1,n,f);fclose(f);word(n);fwrite(b.data(),1,n,stdout);word(CheckV2MVersion(b.data(),n));unsigned char*out=nullptr;int len=0;ConvertV2M(b.data(),n,&out,&len);word(len);fwrite(out,1,len,stdout);delete[]out;}sdClose();}
'''
data=(ROOT/'assets/kkrieger3383.kx').read_bytes();n=struct.unpack_from('<I',data)[0];song=data[4:4+n];pos=4+n+(-n%4);n=struct.unpack_from('<I',data,pos)[0];samples=data[pos+4:pos+4+n];n=struct.unpack_from('<I',samples,4)[0];vfx=samples[8:8+n]
with tempfile.TemporaryDirectory() as tmp:
 tmp=pathlib.Path(tmp);cpp=tmp/'convert.cpp';exe=tmp/'convert';cpp.write_text(support+body+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-Wno-writable-strings','-Wno-unused-value',str(cpp),'-o',str(exe)],check=True)
 versions=[]
 for name in ['v2parms','v2gparms']:
  block=source.split('static const V2PARAM '+name+'[] = {',1)[1].split('};',1)[0]
  versions.append([int(v) for v in re.findall(r'^\s*\{\s*(\d+),',block,re.M)])
 payloads=[song,vfx]
 word=lambda n:struct.pack('<I',n)
 for version in range(7):
  parameters=[i%128 for i,v in enumerate(versions[0]) if v<=version]
  globals=bytes(i%128 for i,v in enumerate(versions[1]) if v<=version)
  midi=word(96)+word(100)+word(0)+word(1)+bytes([0,0,0,60,100])+word(2)+bytes([0,1,0,0,0,0,0,1])+word(0)+word(0)*7+word(0)*15
  patches=[]
  for program in range(2):
   count=3-program;mods=bytes(v for i in range(count) for v in [i,32+i*32,len(parameters)-2-i])
   patches.append(bytes(parameters)+bytes([count])+mods)
  bank=word(8)+word(8+len(patches[0]))+b''.join(patches)
  payloads.append(midi+word(len(globals))+globals+word(len(bank))+bank+word(0))
 paths=[]
 for i,payload in enumerate(payloads):
  p=tmp/f'{i}.v2m';p.write_bytes(payload);paths.append(str(p))
 result=subprocess.check_output([str(exe),*paths])
path=ROOT/'tests/fixtures/v2m-conversion.bin';path.write_bytes(result);print(f'{len(result)} bytes, original conversion metadata, authored music/VFX and seven source versions')
