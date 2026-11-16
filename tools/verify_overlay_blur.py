#!/usr/bin/env python3
"""Original four-sample blend and staged blur, with every graphics pass captured."""
import pathlib,subprocess,tempfile
import verify_overlay_mask as reference
root=reference.root;oracle=reference.oracle;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genoverlay.cpp').read_text(encoding='cp1252')
functions=reference.functions+'\n'+'\n'.join(oracle.function(n) for n in ['void GenOverlayManagerClass::Blend4x(', 'GenOverlayRT *GenOverlayManagerClass::Blend4xOp(', 'GenOverlayRT *GenOverlayManagerClass::BlurOp(', 'void __stdcall Exec_IPP_Blur('])
support=reference.support.replace('#define GENOVER_BLEND4X1 0','#define GENOVER_BLEND4X1 4').replace('struct GenBitmap{int Texture;};','struct GenBitmap{int Texture,XSize,YSize;};').replace('new GenBitmap{100+i}','new GenBitmap{100+i,1<<x,1<<y}')
support=support.replace('void Copy(GenOverlayRT*,GenOverlayRT*,int,unsigned,float);','''void Copy(GenOverlayRT*,GenOverlayRT*,int,unsigned,float);
void Blend4x(GenOverlayRT*,GenOverlayRT*,sFRect*,int,float);GenOverlayRT*Blend4xOp(KOp*,GenOverlayRT*,sFRect*,int,float,int,int);GenOverlayRT*BlurOp(KOp*,GenOverlayRT*,int,float,float,int,int,int);''')
support=support.replace('template<class T>T sMin','template<class T>T sRange(T value,T maximum,T minimum){return value<minimum?minimum:value>maximum?maximum:value;}\ntemplate<class T>T sMin')
support+='''\nvoid dump(){word(systemObject.ViewCalls);word(systemObject.ProjectCalls);word(systemObject.Draws);word(inputCalls);word(records.size());for(auto&r:records){viewport(r.View);word(r.Orthogonal);word(r.Material);for(int t:r.Tex)word(t);for(unsigned color:r.Colors)word(color);for(float f:r.Vertices)real(f);}}\n'''
driver=r'''
int main(){word(512);KEnvironment env;KOp owners[9];float amplitudes[]={-1,0,.015625f,.5f,1,1.99f,4,8};
for(int c=0;c<512;c++){textures.clear();GenOverlayManagerClass pool;pool.Init();GenOverlayManager=&pool;sViewport initial{0,-1,{7,11,640+c%32*23,360+c%32*13}};viewport(initial);pool.SetMasterViewport(initial);int src=c%13,dest=c*7%13,cmode=c%2;float amplify=amplitudes[(c/2)%8];word(src);word(dest);word(cmode);real(amplify);sFRect rects[4];for(int i=0;i<4;i++){rects[i]={float(c%7-3)*.125f,float(i-2)*.5f,float(i+1)*.25f,float(c%11-5)*.0625f};for(float f:{rects[i].x0,rects[i].y0,rects[i].x1,rects[i].y1})real(f);}clearCalls();pool.Blend4x(&pool.RT[src],&pool.RT[dest],rects,cmode,amplify);for(auto r:rects)for(float f:{r.x0,r.y0,r.x1,r.y1})real(f);dump();}
word(32);word(64);float radii[]={-2,0,.25f,1,1.001f,2,8,32,128};
for(int c=0;c<32;c++){textures.clear();GenOverlayManagerClass pool;pool.Init();GenOverlayManager=&pool;sViewport initial{0,-1,{7,11,640+c*23,360+c*13}};viewport(initial);pool.SetMasterViewport(initial);
for(int k=0;k<64;k++){bool reset=k%9==0;if(reset)pool.Reset(&env);word(reset);unsigned src=randomWord()%14,owner=randomWord()%9,size=randomWord()%6,flags=randomWord()%8;int stages=int(randomWord()%8)-2,count=int(randomWord()%5)-1;float radius=radii[randomWord()%9],amplify=amplitudes[randomWord()%8];for(unsigned n:{src,owner,size,flags})word(n);word(stages);word(count);real(radius);real(amplify);clearCalls();requestedInput=src<13?&pool.RT[src]:nullptr;Exec_IPP_Blur(&owners[owner],&env,size,radius,amplify,flags,stages,count);dump();word(pool.LastOutput?unsigned(pool.LastOutput-pool.RT):~0u);for(auto&t:pool.RT){word(t.Owner?unsigned(t.Owner-owners+1):0);word(t.Refs);}
}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'blur.cpp';exe=pathlib.Path(tmp)/'blur';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/overlay-blur.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 512 original blends and 2048 staged blur executions')
