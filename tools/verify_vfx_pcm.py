#!/usr/bin/env python3
"""Original RenderSoundEffects planning, gain ramps, PCM conversion and 3D downmix."""
import pathlib,subprocess,tempfile
ROOT=pathlib.Path(__file__).resolve().parents[1];source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/mainplayer.cpp').read_text(encoding='cp1252');a=source.index('struct VFXEntry');b=source.index('\n#endif',source.index('static void RenderSoundEffects',a));body=source[a:b]
support=r'''
#include <cstdio>
#include <cstring>
#include <vector>
extern "C" double pow(double,double);
using sU8=unsigned char;using sS8=signed char;using sU32=unsigned;using sS16=short;using sInt=int;using sF32=float;using sBool=bool;
#define sVERIFY(x) do{if(!(x))__builtin_trap();}while(0)
#define sMAKE4(a,b,c,d) ((unsigned)(a)|((unsigned)(b)<<8)|((unsigned)(c)<<16)|((unsigned)(d)<<24))
template<class T>T sMin(T a,T b){return a<b?a:b;}template<class T>T sRange(T a,T high,T low){return a<low?low:a>high?high:a;}
double sFPow(double a,double b){return pow(a,b);}void word(unsigned n){fwrite(&n,4,1,stdout);}
struct KDoc{struct{int Count=0;}Ops;};
struct System {void Progress(int,int){}void SampleAdd(short*pcm,int n,int buffers,int handle,bool is3d){word(n);word(buffers);word(handle);word(is3d);fwrite(pcm,2,n*2,stdout);if(is3d)for(int i=0;i<n;i++){short mono=(pcm[i*2]+pcm[i*2+1])>>1;fwrite(&mono,2,1,stdout);}}}system_object;System*sSystem=&system_object;
struct Synth {unsigned seed=1234;bool Open(void*){return true;}void Close(){}void Play(unsigned n){word(n);}void Render(float*out,unsigned n){word(n);for(unsigned i=0;i<n*2;i++){seed=seed*1664525+1013904223;out[i]=((int)((seed>>16)%4096)-2048)/512.f;}fwrite(out,4,n*2,stdout);}}sound_object;Synth*Sound=&sound_object;
'''
driver=r'''
int main(){unsigned lengths[]={1,7,64,128,999,1000,1001,70000};std::vector<unsigned char>data(16+32*16);auto*w=(unsigned*)data.data();w[0]=sMAKE4('V','F','X','0');w[1]=0;w[2]=44100;w[3]=32;auto*entries=(VFXEntry*)(data.data()+16);for(unsigned i=0;i<32;i++){auto&e=entries[i];e.MajorId=i%4;e.MinorId=i*7%256;e.Gain=(int)(i*17%256)-128;e.StartPos=i*12345;e.EndPos=e.StartPos+lengths[i%8];e.LoopSize=i%3==0?lengths[i%8]/2:0;}word(data.size());fwrite(data.data(),1,data.size(),stdout);KDoc doc;RenderSoundEffects(&doc,data.data());}
'''
with tempfile.TemporaryDirectory() as tmp:
 tmp=pathlib.Path(tmp);cpp=tmp/'vfx.cpp';exe=tmp/'vfx';cpp.write_text(support+body+driver);subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off','-fno-builtin',str(cpp),'-o',str(exe)],check=True);result=subprocess.check_output([str(exe)])
path=ROOT/'tests/fixtures/vfx-pcm.bin';path.write_bytes(result);print(f'{len(result)} bytes; 32 original VFX plans and PCM/downmix blocks')
