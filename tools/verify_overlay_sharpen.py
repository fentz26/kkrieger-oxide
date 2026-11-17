#!/usr/bin/env python3
"""Unchanged Exec_IPP_Sharpen, including blur passes and target ownership."""
import pathlib, subprocess, tempfile
import verify_overlay_blur as reference
root=reference.root
oracle=reference.oracle
functions=reference.functions+'\n'+oracle.function('void __stdcall Exec_IPP_Sharpen(')
support=reference.support+'\n#define GENOVER_SHARPEN 7\n'
driver=r'''
int main(){word(32);word(64);KOp owners[9];KEnvironment env;
float radii[]={-2,0,.25f,1,1.001f,2,8,32,128};
for(int c=0;c<32;c++){textures.clear();GenOverlayManagerClass pool;pool.Init();GenOverlayManager=&pool;pool.Mtrl[7]->Color[0]=0x00ffffff;
sViewport initial{0,-1,{7,11,640+c*23,360+c*13}};viewport(initial);pool.SetMasterViewport(initial);
for(int k=0;k<64;k++){bool reset=k%9==0;if(reset)pool.Reset(&env);word(reset);
unsigned src=randomWord()%14,owner=randomWord()%9,size=randomWord()%6,amplify=randomWord();int stages=int(randomWord()%8)-2,count=int(randomWord()%5)-1;float radius=radii[randomWord()%9];
for(unsigned n:{src,owner,size,amplify})word(n);word(stages);word(count);real(radius);
clearCalls();requestedInput=src<13?&pool.RT[src]:nullptr;Exec_IPP_Sharpen(&owners[owner],&env,size,radius,amplify,stages,count);dump();
word(pool.LastOutput?unsigned(pool.LastOutput-pool.RT):~0u);for(auto&t:pool.RT){word(t.Owner?unsigned(t.Owner-owners+1):0);word(t.Refs);}
}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'sharpen.cpp';exe=pathlib.Path(tmp)/'sharpen'
 cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/overlay-sharpen.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 2048 original staged sharpen executions')
