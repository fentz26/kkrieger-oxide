#!/usr/bin/env python3
"""Unchanged FXQuad, Copy and Exec_IPP_Copy with captured graphics calls."""
import pathlib,subprocess,tempfile
import verify_overlay_pool as reference
root=reference.root;oracle=reference.oracle;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genoverlay.cpp').read_text(encoding='cp1252')
functions=reference.functions+'\n'+'\n'.join(oracle.function(n) for n in ['void GenOverlayManagerClass::FXQuad(', 'void GenOverlayManagerClass::Copy(', 'void __stdcall Exec_IPP_Copy('])
support=reference.support.replace('#define sPLAYER 1','#define sPLAYER 1\n#define __stdcall\n#define sMEO_NORMALISED 1\n#define GENOVER_COPY0 27')
support=support.replace('struct KOp{};struct KEnvironment{};','struct KEnvironment{};struct KOp{void ExecInputs(KEnvironment*);};\nstruct sFRect{float x0,y0,x1,y1;void Init(float a,float b,float c,float d){x0=a;y0=b;x1=c;y1=d;}};')
support=support.replace('struct System{','struct sMaterialEnv;\nstruct System{\n sViewport View{};float Vertices[32]{};int ViewCalls=0,ProjectCalls=0,Draws=0,Orthogonal=-1,MaterialId=-1,Texture=-1;unsigned Color=0;\n void SetViewport(sViewport v){View=v;ViewCalls++;}void SetViewProject(sMaterialEnv*);\n void GeoBegin(int,int,int,float**v,int){*v=Vertices;}void GeoEnd(int){}void GeoDraw(int){Draws++;}\n')
support=support.replace('struct sMaterialEnv{void Init(){}};struct Material{int Unbind=0,Bind=0;void SetTex(int,int){Unbind++;}void Set(sMaterialEnv&){Bind++;}};', '''struct sMaterialEnv{int Orthogonal=0;void Init(){Orthogonal=0;}};
void System::SetViewProject(sMaterialEnv*e){ProjectCalls++;Orthogonal=e->Orthogonal;}
struct Material{int Id=0,Unbind=0,Bind=0;unsigned Color[1]{};void SetTex(int,int t){Unbind++;sSystem->Texture=t;}void Set(sMaterialEnv&){Bind++;sSystem->MaterialId=Id;sSystem->Color=Color[0];}};
using sMaterial=Material;''')
support=support.replace('Material material,*Mtrl[1]={&material};','''Material material,materials[36];Material*Mtrl[36];int GeoDouble=0;
GenOverlayManagerClass(){for(int i=0;i<36;i++){materials[i].Id=i;Mtrl[i]=&materials[i];}}
void FXQuad(Material*,float*,sFRect*,float*,sFRect*);
void FXQuad(int m,float*s0=nullptr,sFRect*u0=nullptr,float*s1=nullptr,sFRect*u1=nullptr){FXQuad(Mtrl[m],s0,u0,s1,u1);}
void Copy(GenOverlayRT*,GenOverlayRT*,int,unsigned,float);''')
support+='''\nGenOverlayManagerClass*GenOverlayManager;GenOverlayRT*requestedInput;int inputCalls;
void KOp::ExecInputs(KEnvironment*){inputCalls++;GenOverlayManager->LastOutput=requestedInput;}
void clearCalls(){systemObject.ViewCalls=systemObject.ProjectCalls=systemObject.Draws=0;systemObject.Orthogonal=systemObject.MaterialId=systemObject.Texture=-1;systemObject.Color=0;inputCalls=0;}
void draw(){word(systemObject.ViewCalls);word(systemObject.ProjectCalls);word(systemObject.Draws);word(systemObject.Orthogonal);word(systemObject.MaterialId);word(systemObject.Texture);word(systemObject.Color);if(systemObject.ViewCalls)viewport(systemObject.View);if(systemObject.Draws)for(auto f:systemObject.Vertices)real(f);word(inputCalls);}
'''
driver=r'''
int main(){word(512);for(int c=0;c<512;c++){GenOverlayManagerClass pool;unsigned mask=c%16;float scales[4]={c*.001f-.25f,.5f-c*.0005f,-.75f+c*.0001f,1.25f};sFRect uv[2]={{-.1f,c*.001f,.8f,1.1f},{c*.0001f,-.2f,.9f,.7f}};word(mask);word(c%36);for(float f:scales)real(f);for(auto r:uv)for(float f:{r.x0,r.y0,r.x1,r.y1})real(f);clearCalls();pool.FXQuad(c%36,mask&1?scales:nullptr,mask&2?&uv[0]:nullptr,mask&4?scales+2:nullptr,mask&8?&uv[1]:nullptr);draw();}
word(32);word(64);KOp owners[9];KEnvironment env;
for(int c=0;c<32;c++){textures.clear();GenOverlayManagerClass pool;pool.Init();GenOverlayManager=&pool;sViewport initial{0,-1,{7,11,640+c*23,360+c*13}};viewport(initial);pool.SetMasterViewport(initial);
for(int k=0;k<64;k++){bool reset=k%7==0;if(reset)pool.Reset(&env);word(reset);unsigned type=randomWord()%2,src=randomWord()%14,dest=randomWord()%13,owner=randomWord()%9,size=randomWord()%6,mode=randomWord()%6,color=randomWord();int count=int(randomWord()%5)-1;float zoom=float(int(randomWord()%41)-20)*.125f;
for(unsigned n:{type,src,dest,owner,size,mode,color})word(n);word(count);real(zoom);clearCalls();if(type==0)pool.Copy(&pool.RT[src%13],&pool.RT[dest],mode,color,zoom);else{requestedInput=src<13?&pool.RT[src]:nullptr;Exec_IPP_Copy(&owners[owner],&env,size,color,zoom,count);}draw();
word(pool.LastOutput?unsigned(pool.LastOutput-pool.RT):~0u);for(int i=0;i<13;i++){auto&t=pool.RT[i];word(t.Owner?unsigned(t.Owner-owners+1):0);word(t.Refs);}
}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'copy.cpp';exe=pathlib.Path(tmp)/'copy';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/overlay-copy.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 512 original FXQuad calls and 2048 copy operations')
