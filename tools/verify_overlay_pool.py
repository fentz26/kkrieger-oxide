#!/usr/bin/env python3
"""Original player IPP target initialization, allocation and viewport methods."""
import pathlib, subprocess, tempfile
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger';source=(base/'genoverlay.cpp').read_text(encoding='cp1252');oracle.source=source
names=['void GenOverlayManagerClass::Reset(', 'GenOverlayRT *GenOverlayManagerClass::Get(', 'GenOverlayRT *GenOverlayManagerClass::Alloc(', 'GenOverlayRT *GenOverlayManagerClass::Find(', 'void GenOverlayManagerClass::Free(', 'void GenOverlayManagerClass::PrepareViewport(', 'void GenOverlayManagerClass::SetMasterViewport(', 'void GenOverlayManagerClass::GetMasterViewport(']
functions='\n'.join(oracle.function(name) for name in names)
a=source.index('  static sInt sizes[GENOVER_RTSIZES]');b=source.index('\n  };',a)+5;sizes=source[a:b]
a=source.index('  rtd = RT;');b=source.index('\n  GeoDouble =',a);init=source[a:b]
header=(base/'genoverlay.hpp').read_text(encoding='cp1252');a=header.index('struct GenOverlayRT\n');b=header.index('\n};',a)+3;target_type=header[a:b]
oracle.source=(base/'_start.cpp').read_text(encoding='cp1252');functions+='\n'+oracle.function('void sViewport::InitTex(')
support=r'''
#include <cstdio>
#include <vector>
#include <algorithm>
using sInt=int;using sF32=float;using sU32=unsigned;
#define sPLAYER 1
#define sDUMP_IPPALLOC 0
#define sVERIFY(x) ((void)0)
#define GENOVER_RTSIZES 4
#define GENOVER_RTPERSIZE 3
#define GENOVER_RTCOUNT 12
#define GENOVER_BLEND4X1 0
#define sINVALID -1
#define sPS_11 1
#define sPS_20 20
#define sTF_A8R8G8B8 1
template<class T>T sMin(T a,T b){return std::min(a,b);}
struct KOp{};struct KEnvironment{};
struct sRect{int x0,y0,x1,y1;int XSize(){return x1-x0;}int YSize(){return y1-y0;}};
struct sViewport{int Screen,RenderTarget;sRect Window;void InitTex(int);};
struct Texture{int Width,Height;};std::vector<Texture>textures;
struct System{void GetTextureSize(int id,int&w,int&h){w=textures[id-100].Width;h=textures[id-100].Height;}}systemObject,*sSystem=&systemObject;
struct GenBitmap{int Texture;};
GenBitmap*Bitmap_RenderTarget(int x,int y,int){int i=textures.size();textures.push_back({1<<x,1<<y});return new GenBitmap{100+i};}
struct sMaterialEnv{void Init(){}};struct Material{int Unbind=0,Bind=0;void SetTex(int,int){Unbind++;}void Set(sMaterialEnv&){Bind++;}};
'''+target_type+r'''
struct GenOverlayManagerClass{GenOverlayRT RT[13]{};int CurrentShader=1;Material material,*Mtrl[1]={&material};GenOverlayRT*LastOutput=nullptr;KEnvironment*KEnv=nullptr;sViewport Master;
void Init();void Reset(KEnvironment*);GenOverlayRT*Get(int,int);GenOverlayRT*Alloc(KOp*,int,int);GenOverlayRT*Find(KOp*);void Free(GenOverlayRT*);void PrepareViewport(GenOverlayRT*,sViewport&);void SetMasterViewport(sViewport&);void GetMasterViewport(sViewport&);};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void viewport(sViewport v){for(int n:{v.Screen,v.RenderTarget,v.Window.x0,v.Window.y0,v.Window.x1,v.Window.y1})word(n);}
unsigned seed=1;unsigned randomWord(){return seed=seed*1664525u+1013904223u;}
'''
functions+='\nvoid GenOverlayManagerClass::Init(){sInt i,j;GenOverlayRT*rtd;\n'+sizes+init+'\n}\n'
driver=r'''
int main(){word(32);word(128);KOp owners[9];KEnvironment env;
for(int c=0;c<32;c++){textures.clear();GenOverlayManagerClass pool;pool.Init();sViewport initial{0,-1,{0,0,640+c*17,360+c*9}};viewport(initial);pool.SetMasterViewport(initial);
for(int k=0;k<128;k++){
 unsigned code=randomWord()%7,owner=randomWord()%10,size=randomWord()%7,index=randomWord()%13;int count=int(randomWord()%5)-1;
 sViewport vp{int(c%3),-1,{int(randomWord()%71),int(randomWord()%61),int(randomWord()%1800),int(randomWord()%1000)}};
 word(code);word(owner);word(size);word(index);word(count);viewport(vp);
 GenOverlayRT*result=nullptr;
 switch(code){case 0:pool.Reset(&env);break;case 1:result=pool.Alloc(owner?&owners[owner-1]:nullptr,size,count);break;case 2:result=pool.Get(size,index%3);break;case 3:result=pool.Find(owner?&owners[owner-1]:nullptr);break;case 4:pool.Free(&pool.RT[index]);break;case 5:pool.SetMasterViewport(vp);break;case 6:pool.PrepareViewport(&pool.RT[index],vp);break;}
 word(result?unsigned(result-pool.RT):~0u);viewport(vp);
 for(int i=0;i<13;i++){auto&t=pool.RT[i];word(t.Owner?unsigned(t.Owner-owners+1):0);word(t.Size);word(t.Refs);if(i<12){real(t.ScaleUV[0]);real(t.ScaleUV[1]);}}
 sViewport master;pool.GetMasterViewport(master);viewport(master);word(pool.LastOutput?unsigned(pool.LastOutput-pool.RT):~0u);word(pool.material.Unbind);word(pool.material.Bind);
 if(k%11==0)pool.LastOutput=&pool.RT[index];
}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'pool.cpp';exe=pathlib.Path(tmp)/'pool';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/overlay-pool.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 4096 original IPP target operations')
