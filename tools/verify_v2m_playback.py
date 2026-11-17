#!/usr/bin/env python3
"""Original V2M Play/Stop/Render scheduling with deterministic synth adapters."""
import pathlib,struct,subprocess,tempfile
ROOT=pathlib.Path(__file__).resolve().parents[1];base=ROOT/'.reference/fr_public/werkkzeug3_kkrieger';source=(base/'_viruz2.cpp').read_text(encoding='cp1252');header=(base/'_viruz2.hpp').read_text(encoding='cp1252')
def function(signature):
 a=source.index(signature);p=source.index('{',a);depth=1;b=p+1
 while depth:
  depth+=(source[b]=='{')-(source[b]=='}');b+=1
 return source[a:b]
a=header.index('\tstruct V2MBase');b=header.index('\n\n\n\t// player state',a);base_type=header[a:b];a=header.index('\tstruct PlayerState');b=header.index('\n\n\t// member variables',a);state_type=header[a:b]
a=source.index('#define GETDELTA');b=source.index('\n\nnamespace',a);macros=source[a:b]
functions='\n'.join(function(s) for s in ['sBool CV2MPlayer::InitBase(', 'void CV2MPlayer::Reset()', 'void CV2MPlayer::Tick()', 'sBool CV2MPlayer::Open(', 'void CV2MPlayer::Close()', 'void CV2MPlayer::Play(', 'void CV2MPlayer::Stop(', 'sBool CV2MPlayer::Render('])
import re
# The x86 integer arithmetic is expressed with the same signed product/division.
functions=re.sub(r'__asm\s*\{.*?\}',lambda m:'destsmpl=timesamples(a_time,m_samplerate,m_tpc);' if 'destsmpl' in m[0] else 'ftsmpls=timesamples(a_fadetime,m_samplerate,m_tpc);' if 'ftsmpls' in m[0] else 'memset(a_buffer,0,a_len*8);',functions,flags=re.S)
support=r'''
#include <cstdio>
#include <cstring>
#include <vector>
using sU32=unsigned;using sS32=int;using sInt=int;using sU8=unsigned char;using sBool=bool;using sF32=float;
#define RONAN
#define sFALSE false
#define sTRUE true
static unsigned phase=0,hash=0;static std::vector<unsigned>trace;
void synthInit(const void*,int){phase=hash=0;trace.push_back(0);}void synthSetGlobals(const void*){trace.push_back(1);}void synthSetLyrics(const char**){trace.push_back(2);}
void synthProcessMIDI(const void*p){trace.push_back(3);auto*b=(const unsigned char*)p;unsigned n=0;while(b[n]!=0xfd)n++;n++;trace.push_back(n);for(unsigned i=0;i<n;i++){trace.push_back(b[i]);hash=(hash*33)^b[i];}}
void synthRender(float*out,unsigned n){trace.push_back(4);trace.push_back(n);for(unsigned i=0;i<n;i++){phase++;out[i*2]=((phase^hash)&65535)/65536.f-.5f;out[i*2+1]=(((phase*7)^hash)>>8&65535)/65536.f-.5f;}}
unsigned timesamples(unsigned time,unsigned rate,unsigned tpc){return (int)(((long long)(int)time*(int)rate)/(int)tpc);}
void UpdateSampleDelta(unsigned next,unsigned time,unsigned usecs,unsigned div,unsigned*rem,unsigned*delta){auto product=(unsigned long long)(next-time)*usecs;unsigned d=product/div;unsigned r=product%div;unsigned sum=*rem+r;*delta=d+(sum<*rem);*rem=sum;}
class CV2MPlayer{public:
'''+base_type+state_type+r'''
const unsigned m_tpc;V2MBase m_base{};PlayerState m_state{};unsigned m_samplerate=0;int m_timeoffset=0;unsigned char m_midibuf[4096];float m_fadeval=0,m_fadedelta=0;
CV2MPlayer(unsigned tpc):m_tpc(tpc){}bool InitBase(const void*);void Reset();void Tick();bool Open(const void*,unsigned);void Close();void Play(unsigned=0);void Stop(unsigned=0);bool Render(float*,unsigned);
};
void word(unsigned n){fwrite(&n,4,1,stdout);}void snapshot(CV2MPlayer&p){auto&s=p.m_state;for(unsigned n:{unsigned(p.m_base.valid),unsigned(s.state),s.time,s.nexttime,s.usecs,s.bar,s.beat,s.tick,s.cursmpl,s.smpldelta,s.smplrem,unsigned(p.m_timeoffset),phase,hash})word(n);fwrite(&p.m_fadeval,4,1,stdout);fwrite(&p.m_fadedelta,4,1,stdout);word(trace.size());for(auto n:trace)word(n);trace.clear();}
'''
driver=r'''
int main(int argc,char**argv){word((argc-1)*3);for(int a=1;a<argc;a++){FILE*f=fopen(argv[a],"rb");fseek(f,0,SEEK_END);int size=ftell(f);rewind(f);std::vector<unsigned char>data(size);fread(data.data(),1,size,f);fclose(f);for(unsigned rate:{22050,44100,48000}){unsigned tpc=rate==48000?300:1000;word(size);fwrite(data.data(),1,size,stdout);word(rate);word(tpc);trace.clear();CV2MPlayer p(tpc);p.Open(data.data(),rate);snapshot(p);unsigned ops[][2]={{0,7},{1,0},{0,1},{0,127},{0,128},{2,10},{0,256},{0,256},{2,0},{1,10},{0,17},{1,1000000},{0,13},{2,1},{0,64},{3,0},{0,5}};word(17);for(auto&op:ops){word(op[0]);word(op[1]);if(op[0]==0){std::vector<float>buffer(op[1]*4,.375f);word(p.Render(buffer.data(),op[1]));fwrite(buffer.data(),4,buffer.size(),stdout);}else if(op[0]==1)p.Play(op[1]);else if(op[0]==2)p.Stop(op[1]);else p.Close();snapshot(p);}}}}
'''
data=(ROOT/'assets/kkrieger3383.kx').read_bytes();n=struct.unpack_from('<I',data)[0];song=data[4:4+n];pos=4+n+(-n%4);n=struct.unpack_from('<I',data,pos)[0];samples=data[pos+4:pos+4+n];n=struct.unpack_from('<I',samples,4)[0];vfx=samples[8:8+n]
w=lambda n:struct.pack('<I',n)
short=w(96)+w(1)+w(0)+w(2)+bytes([0,1,0,0,0,0,60,0,100,156])+w(1)+bytes(4)+w(0)+w(0)*7+w(0)*15+w(0)+w(0)+w(0)
with tempfile.TemporaryDirectory() as tmp:
 tmp=pathlib.Path(tmp);cpp=tmp/'playback.cpp';exe=tmp/'playback';cpp.write_text(support+macros+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
 paths=[]
 for i,payload in enumerate([song,vfx,short]):
  p=tmp/f'{i}.v2m';p.write_bytes(payload);paths.append(str(p))
 result=subprocess.check_output([str(exe),*paths])
path=ROOT/'tests/fixtures/v2m-playback.bin';path.write_bytes(result);print(f'{len(result)} bytes; nine original playback scenarios, 153 operations')
