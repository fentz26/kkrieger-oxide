#!/usr/bin/env python3
"""Original IPP Select, including screen refs that survive manager Reset."""
import pathlib,subprocess,tempfile
import verify_overlay_copy as reference
root=reference.root;oracle=reference.oracle;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genoverlay.cpp').read_text(encoding='cp1252')
functions=reference.functions+'\n'+oracle.function('void __stdcall Exec_IPP_Select(')
support=reference.support
driver=r'''
int main(){word(64);word(32);KOp owner;KEnvironment env;
for(int c=0;c<64;c++){textures.clear();GenOverlayManagerClass pool;pool.Init();GenOverlayManager=&pool;for(int i=0;i<13;i++){pool.RT[i].Refs=(i+c)%4;word(pool.RT[i].Refs);}
for(int k=0;k<32;k++){bool reset=k%8==0;if(reset)pool.Reset(&env);word(reset);unsigned src=randomWord()%14;int count=int(randomWord()%10)-3;word(src);word(count);clearCalls();requestedInput=src<13?&pool.RT[src]:nullptr;Exec_IPP_Select(&owner,&env,count);word(inputCalls);word(pool.LastOutput?unsigned(pool.LastOutput-pool.RT):~0u);for(auto&t:pool.RT)word(t.Refs);word(systemObject.Draws);}
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'select.cpp';exe=pathlib.Path(tmp)/'select';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/overlay-select.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 2048 original IPP select executions')
