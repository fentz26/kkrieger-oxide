#!/usr/bin/env python3
"""Original Ronan phoneme decode, frame setup and complete resonator processing."""
import pathlib,subprocess,tempfile
ROOT=pathlib.Path(__file__).resolve().parents[1]
source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2.cpp').read_text(encoding='cp1252')
body=source[source.index('#pragma pack (push, 1)'):source.index('#define GETDELTA')]
support='''#include <cstdio>
extern "C" {double exp(double);double pow(double,double);double cos(double);double atan(double);double fabs(double);long lrintf(float);}
#include <cstring>
#define RONAN
#define __stdcall
using sU8=unsigned char;using sS8=signed char;using sS32=int;using sU32=unsigned;using sInt=int;using sF32=float;
double sFExp(double x){return exp(x);}double sFPow(double a,double b){return pow(a,b);}int sFtol(float x){return (int)lrintf(x);}
'''
driver=r'''
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float n){fwrite(&n,4,1,stdout);}
void frame(const Ronan::syVRonan&f){fwrite(&f,4,25,stdout);}
void state(){auto&w=Ronan::workspace;frame(w);for(auto&r:w.res){real(r.p1);real(r.p2);}real(w.lastin2);word(w.nseed);real(w.nout);real(w.hpb1);real(w.hpb2);real(w.peq1.p1);real(w.peq1.p2);}
int main(){ronanCBSetSR(44100);ronanCBInit();word(69);fwrite(Ronan::phonemes,4,69*19,stdout);word(NSYLS);fwrite(Ronan::syls,8,NSYLS,stdout);word(128);word(3);unsigned seed=1234;for(unsigned c=0;c<128;c++){unsigned rate[]={22050,44100,48000,96000};unsigned n[]={1,7,64,128,280};unsigned sr=rate[c%4],count=n[c%5],p1=c%69,p2=(c*7)%69;float x=(c%129)/128.f,pitch=.5f+(c%8)*.25f;ronanCBSetSR(sr);ronanCBInit();Ronan::pitch=pitch;word(sr);word(count);word(p1);word(p2);real(x);real(pitch);auto&w=Ronan::workspace;Ronan::SetFrame(Ronan::phonemes[p1],Ronan::phonemes[p2],x,w);Ronan::SetFrame(Ronan::phonemes[p2],Ronan::phonemes[p1],1.f-x,w.newframe);frame(w);frame(w.newframe);for(unsigned block=0;block<3;block++){float data[560];for(unsigned i=0;i<count*2;i++){seed=seed*1664525+1013904223;data[i]=((int)((seed>>16)%1024)-512)/512.f;}fwrite(data,4,count*2,stdout);ronanCBProcess(data,count);state();fwrite(data,4,count*2,stdout);}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 tmp=pathlib.Path(tmp);cpp=tmp/'speech.cpp';exe=tmp/'speech';cpp.write_text(support+body+driver)
 # _types.hpp declares only double math functions. Avoid modern float overloads
 # and preserve those original double calls in the reference build.
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off','-fno-builtin','-Wno-register','-Wno-parentheses',str(cpp),'-o',str(exe)],check=True)
 result=subprocess.check_output([str(exe)])
path=ROOT/'tests/fixtures/synth-speech-dsp.bin';path.write_bytes(result);print(f'{len(result)} bytes; 69 phonemes, 128 frame setups and 384 speech DSP blocks')
