#!/usr/bin/env python3
"""Original IPP mask with both material/texture/geometry passes captured."""
import pathlib,subprocess,tempfile
import verify_overlay_merge as reference
root=reference.root;oracle=reference.oracle;source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genoverlay.cpp').read_text(encoding='cp1252');oracle.source=source
functions=reference.functions+'\n'+oracle.function('void __stdcall Exec_IPP_Mask(')
support=reference.support.replace('#define GENOVER_MERGE0 16','#define GENOVER_MERGE0 16\n#define GENOVER_MASK1 25\n#define GENOVER_MASK2 26').replace('requestedInputs[2]','requestedInputs[3]')
support=support.replace('struct sMaterialEnv;','''struct Record{sViewport View;int Orthogonal,Material,Tex[3];unsigned Colors[2];float Vertices[32];};std::vector<Record>records;
struct sMaterialEnv;''').replace('int Texture1=-1;','int Texture1=-1,Texture2=-1;')
support=support.replace('void GeoDraw(int){Draws++;}', '''void GeoDraw(int){Draws++;Record r;r.View=View;r.Orthogonal=Orthogonal;r.Material=MaterialId;r.Tex[0]=Texture;r.Tex[1]=Texture1;r.Tex[2]=Texture2;r.Colors[0]=Color;r.Colors[1]=Color2;for(int i=0;i<32;i++)r.Vertices[i]=Vertices[i];records.push_back(r);}''')
support=support.replace('unsigned Color[3]{};','unsigned Color[3]{~0u,~0u,~0u};int Tex[3]{-1,-1,-1};').replace('void SetTex(int slot,int t){Unbind++;if(slot==0)sSystem->Texture=t;else if(slot==1)sSystem->Texture1=t;}','void SetTex(int slot,int t){Unbind++;Tex[slot]=t;}').replace('sSystem->Color2=Color[2];','sSystem->Color2=Color[2];sSystem->Texture=Tex[0];sSystem->Texture1=Tex[1];sSystem->Texture2=Tex[2];').replace('void clearCalls(){','void clearCalls(){records.clear();')
a=source.index('  Mtrl[GENOVER_MASK1] = new');b=source.index('  for(i=0;i<6;i++)',a)
mask_state='\n'.join(line.replace('Mtrl[','pool.Mtrl[') for line in source[a:b].splitlines() if '->Color[' in line or '->SetTex(' in line)
driver=r'''
int main(){word(32);word(128);KOp owners[9];KEnvironment env;
for(int c=0;c<32;c++){textures.clear();GenOverlayManagerClass pool;pool.Init();GenOverlayManager=&pool;
'''+mask_state+r'''
sViewport initial{0,-1,{7,11,640+c*23,360+c*13}};viewport(initial);pool.SetMasterViewport(initial);
for(int k=0;k<128;k++){bool reset=k%11==0;if(reset)pool.Reset(&env);word(reset);unsigned src0=randomWord()%14,src1=randomWord()%14,src2=randomWord()%14,owner=randomWord()%9,size=randomWord()%6;if(k%9==0)src1=src2=src0;int count=int(randomWord()%5)-1;
for(unsigned n:{src0,src1,src2,owner,size})word(n);word(count);clearCalls();requestedInputs[0]=src0<13?&pool.RT[src0]:nullptr;requestedInputs[1]=src1<13?&pool.RT[src1]:nullptr;requestedInputs[2]=src2<13?&pool.RT[src2]:nullptr;Exec_IPP_Mask(&owners[owner],&env,size,count);
word(systemObject.ViewCalls);word(systemObject.ProjectCalls);word(systemObject.Draws);word(inputCalls);word(inputTrace);word(records.size());for(auto&r:records){viewport(r.View);word(r.Orthogonal);word(r.Material);for(int t:r.Tex)word(t);for(unsigned color:r.Colors)word(color);for(float f:r.Vertices)real(f);}
word(pool.LastOutput?unsigned(pool.LastOutput-pool.RT):~0u);for(auto&t:pool.RT){word(t.Owner?unsigned(t.Owner-owners+1):0);word(t.Refs);}
}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'mask.cpp';exe=pathlib.Path(tmp)/'mask';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/overlay-mask.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 4096 original two-pass IPP mask executions')
