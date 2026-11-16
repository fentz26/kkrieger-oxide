#!/usr/bin/env python3
"""Original two-input IPP merge order, cache, dual UVs and reference effects."""
import pathlib,subprocess,tempfile
import verify_overlay_color as reference
root=reference.root;oracle=reference.oracle;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genoverlay.cpp').read_text(encoding='cp1252')
functions=reference.functions+'\n'+oracle.function('void __stdcall Exec_IPP_Merge(')
support=reference.support.replace('#define GENOVER_COLOR0 8','#define GENOVER_COLOR0 8\n#define GENOVER_MERGE0 16').replace('struct KOp{void ExecInputs(KEnvironment*);};','struct KOp{void ExecInputs(KEnvironment*);void ExecInput(KEnvironment*,int);};').replace('unsigned Color=0,Color2=0;','int Texture1=-1;unsigned Color=0,Color2=0;').replace('void SetTex(int,int t){Unbind++;sSystem->Texture=t;}','void SetTex(int slot,int t){Unbind++;if(slot==0)sSystem->Texture=t;else if(slot==1)sSystem->Texture1=t;}').replace('int inputCalls;','int inputCalls,inputTrace;GenOverlayRT*requestedInputs[2];').replace('systemObject.Color=systemObject.Color2=0;inputCalls=0;','systemObject.Color=systemObject.Color2=0;systemObject.Texture1=-1;inputCalls=inputTrace=0;')
support+='\nvoid KOp::ExecInput(KEnvironment*,int i){inputCalls++;inputTrace=(inputTrace<<4)|(i+1);GenOverlayManager->LastOutput=requestedInputs[i];}\n'
driver=r'''
int main(){word(32);word(128);KOp owners[9];KEnvironment env;
for(int c=0;c<32;c++){textures.clear();GenOverlayManagerClass pool;pool.Init();GenOverlayManager=&pool;sViewport initial{0,-1,{7,11,640+c*23,360+c*13}};viewport(initial);pool.SetMasterViewport(initial);
for(int k=0;k<128;k++){bool reset=k%11==0;if(reset)pool.Reset(&env);word(reset);unsigned src0=randomWord()%14,src1=randomWord()%14,owner=randomWord()%9,size=randomWord()%6,operation=randomWord()%9,alpha=randomWord()%256,amplify=randomWord();if(k%9==0)src1=src0;int count=int(randomWord()%5)-1;
for(unsigned n:{src0,src1,owner,size,operation,alpha,amplify})word(n);word(count);clearCalls();requestedInputs[0]=src0<13?&pool.RT[src0]:nullptr;requestedInputs[1]=src1<13?&pool.RT[src1]:nullptr;Exec_IPP_Merge(&owners[owner],&env,size,operation,alpha,amplify,count);draw();word(systemObject.Color2);word(systemObject.Texture1);word(inputTrace);
word(pool.LastOutput?unsigned(pool.LastOutput-pool.RT):~0u);for(int i=0;i<13;i++){auto&t=pool.RT[i];word(t.Owner?unsigned(t.Owner-owners+1):0);word(t.Refs);}
}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'merge.cpp';exe=pathlib.Path(tmp)/'merge';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/overlay-merge.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 4096 original IPP merge executions')
