#!/usr/bin/env python3
"""Original Exec_IPP_Color cache, input execution, draw and reference behavior."""
import pathlib,subprocess,tempfile
import verify_overlay_copy as reference
root=reference.root;oracle=reference.oracle;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genoverlay.cpp').read_text(encoding='cp1252')
functions=reference.functions+'\n'+oracle.function('void __stdcall Exec_IPP_Color(')
support=reference.support.replace('#define GENOVER_COPY0 27','#define GENOVER_COPY0 27\n#define GENOVER_COLOR0 8').replace('unsigned Color[1]{}','unsigned Color[3]{}').replace('unsigned Color=0;','unsigned Color=0,Color2=0;').replace('sSystem->Color=Color[0];','sSystem->Color=Color[0];sSystem->Color2=Color[2];').replace('systemObject.Color=0;inputCalls=0;','systemObject.Color=systemObject.Color2=0;inputCalls=0;')
driver=r'''
int main(){word(32);word(128);KOp owners[9];KEnvironment env;
for(int c=0;c<32;c++){textures.clear();GenOverlayManagerClass pool;pool.Init();GenOverlayManager=&pool;sViewport initial{0,-1,{7,11,640+c*23,360+c*13}};viewport(initial);pool.SetMasterViewport(initial);
for(int k=0;k<128;k++){bool reset=k%11==0;if(reset)pool.Reset(&env);word(reset);unsigned src=randomWord()%14,owner=randomWord()%9,size=randomWord()%6,operation=randomWord()%8,color=randomWord(),amplify=randomWord();int count=int(randomWord()%5)-1;
for(unsigned n:{src,owner,size,operation,color,amplify})word(n);word(count);clearCalls();requestedInput=src<13?&pool.RT[src]:nullptr;Exec_IPP_Color(&owners[owner],&env,size,color,operation,amplify,count);draw();word(systemObject.Color2);
word(pool.LastOutput?unsigned(pool.LastOutput-pool.RT):~0u);for(int i=0;i<13;i++){auto&t=pool.RT[i];word(t.Owner?unsigned(t.Owner-owners+1):0);word(t.Refs);}
}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'color.cpp';exe=pathlib.Path(tmp)/'color';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/overlay-color.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 4096 original IPP color executions')
