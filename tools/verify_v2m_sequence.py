#!/usr/bin/env python3
"""Original V2M Reset/Tick MIDI bytes and complete timing progression."""
import pathlib,subprocess,tempfile
import verify_v2m_base as reference
root=reference.root;oracle=reference.oracle;base=reference.base
source=(base/'_viruz2.cpp').read_text(encoding='cp1252');oracle.source=source
functions='\n'.join(oracle.function(n) for n in ['sBool CV2MPlayer::InitBase(', 'void CV2MPlayer::Reset()', 'void CV2MPlayer::Tick()'])
h=(base/'_viruz2.hpp').read_text(encoding='cp1252');a=h.index('\tstruct PlayerState');b=h.index('\n\n\t// member variables',a);state=h[a:b]
a=source.index('#define GETDELTA');b=source.index('\n\nnamespace',a);macros=source[a:b]
support='''
#include <cstdio>
#include <vector>
using sU32=unsigned;using sInt=int;using sU8=unsigned char;using sBool=bool;
#define RONAN
#define sFALSE false
#define sTRUE true
void synthInit(const void*,int){}void synthSetGlobals(const void*){}void synthSetLyrics(const char**){}
static std::vector<unsigned char>midi;
void synthProcessMIDI(const void*p){auto*d=(const unsigned char*)p;midi.clear();do{midi.push_back(*d);}while(*d++!=0xfd);}
class CV2MPlayer{public:
'''+reference.base_type+state+'''
V2MBase m_base{};PlayerState m_state{};unsigned m_samplerate=44100;unsigned char m_midibuf[4096];bool InitBase(const void*);void Reset();void Tick();};
void word(unsigned n){fwrite(&n,4,1,stdout);}
void snapshot(CV2MPlayer&p){auto&s=p.m_state;for(unsigned n:{s.time,s.nexttime,s.gnr,s.gnt,s.usecs,s.num,s.den,s.tpq,s.bar,s.beat,s.tick,unsigned(s.state==CV2MPlayer::PlayerState::PLAYING)})word(n);}
'''
driver=r'''
int main(int argc,char**argv){word(argc-1);for(int a=1;a<argc;a++){FILE*f=fopen(argv[a],"rb");fseek(f,0,SEEK_END);int size=ftell(f);rewind(f);std::vector<unsigned char>d(size);fread(d.data(),1,size,f);fclose(f);word(size);fwrite(d.data(),1,size,stdout);CV2MPlayer p;p.InitBase(d.data());p.Reset();p.m_state.state=CV2MPlayer::PlayerState::PLAYING;snapshot(p);unsigned ticks=0;while(p.m_state.state==CV2MPlayer::PlayerState::PLAYING){if(ticks++>50000)return 2;word(1);p.Tick();snapshot(p);word(midi.size());fwrite(midi.data(),1,midi.size(),stdout);}word(0);}}
'''
with tempfile.TemporaryDirectory() as tmp:
 tmp=pathlib.Path(tmp);cpp=tmp/'sequence.cpp';exe=tmp/'sequence';cpp.write_text(support+macros+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2',str(cpp),'-o',str(exe)],check=True)
 paths=[]
 for i,payload in enumerate([reference.song,reference.vfx]):
  p=tmp/f'{i}.v2m';p.write_bytes(payload);paths.append(str(p))
 result=subprocess.check_output([str(exe),*paths])
out=root/'tests/fixtures/v2m-sequence.bin';out.write_bytes(result);print(f'{out}: {len(result)} bytes, complete original song and VFX MIDI timelines')
