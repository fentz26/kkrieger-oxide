#!/usr/bin/env python3
"""Original CV2MPlayer::InitBase offsets for both authored V2M payloads."""
import pathlib,subprocess,tempfile,struct
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
oracle.source=(base/'_viruz2.cpp').read_text(encoding='cp1252');init=oracle.function('sBool CV2MPlayer::InitBase(')
h=(base/'_viruz2.hpp').read_text(encoding='cp1252');a=h.index('\tstruct V2MBase');b=h.index('\n\n\n\t// player state',a);base_type=h[a:b]
support='''
#include <cstdio>
#include <vector>
using sU32=unsigned;using sInt=int;using sU8=unsigned char;using sBool=bool;
#define RONAN
#define sFALSE false
#define sTRUE true
class CV2MPlayer{public:
'''+base_type+'''\nV2MBase m_base{};bool InitBase(const void*);};
void word(unsigned n){fwrite(&n,4,1,stdout);}
'''
driver=r'''
int main(int argc,char**argv){word(argc-1);for(int a=1;a<argc;a++){FILE*f=fopen(argv[a],"rb");fseek(f,0,SEEK_END);int size=ftell(f);rewind(f);std::vector<unsigned char>d(size);fread(d.data(),1,size,f);fclose(f);word(size);fwrite(d.data(),1,size,stdout);CV2MPlayer p;word(p.InitBase(d.data()));auto&b=p.m_base;word(b.timediv);word(b.timediv2);word(b.maxtime);word(b.gdnum);word(b.gptr-d.data());for(auto&c:b.chan){word(c.notenum);if(c.notenum){word(c.noteptr-d.data());word(c.pcnum);word(c.pcptr-d.data());word(c.pbnum);word(c.pbptr-d.data());for(auto&cc:c.ctl){word(cc.ccnum);word(cc.ccptr-d.data());}}}word(b.globals-d.data());word(b.patchmap-d.data());word(b.speechdata?reinterpret_cast<const unsigned char*>(b.speechdata)-d.data():~0u);for(auto text:b.speechptrs){if(!text){word(~0u);}else{unsigned len=0;while(text[len])len++;word(len);fwrite(text,1,len,stdout);}}}}
'''
data=(root/'assets/kkrieger3383.kx').read_bytes();n=struct.unpack_from('<I',data)[0];song=data[4:4+n];pos=4+n+(-n%4);n=struct.unpack_from('<I',data,pos)[0];samples=data[pos+4:pos+4+n];n=struct.unpack_from('<I',samples,4)[0];vfx=samples[8:8+n]
with tempfile.TemporaryDirectory() as tmp:
 tmp=pathlib.Path(tmp);cpp=tmp/'base.cpp';exe=tmp/'base';cpp.write_text(support+init+driver)
 subprocess.run(['clang++','-std=c++17','-O2',str(cpp),'-o',str(exe)],check=True)
 paths=[]
 for i,payload in enumerate([song,vfx]):
  p=tmp/f'{i}.v2m';p.write_bytes(payload);paths.append(str(p))
 result=subprocess.check_output([str(exe),*paths])
out=root/'tests/fixtures/v2m-base.bin';out.write_bytes(result);print(f'{out}: {len(result)} bytes, original song and VFX base tables')
