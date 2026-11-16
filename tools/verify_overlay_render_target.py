#!/usr/bin/env python3
"""Original linked render-target wrapper with captured graphics callbacks."""
import pathlib,subprocess,tempfile
import verify_overlay_copy as reference
root=reference.root;oracle=reference.oracle;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genoverlay.cpp').read_text(encoding='cp1252');functions=reference.functions+'\n'+oracle.function('void __stdcall Exec_IPP_RenderTarget(')
support=reference.support.replace('struct KEnvironment{};struct KOp{void ExecInputs(KEnvironment*);};','struct KEnvironment{};struct GenBitmap;struct KOp{GenBitmap*Cache=nullptr;KOp*Link=nullptr;KOp*GetLink(int){return Link;}void ExecInputs(KEnvironment*);};')
driver=r'''
int main(){word(256);KEnvironment env;for(int c=0;c<256;c++){textures.clear();GenOverlayManagerClass pool;pool.Init();GenOverlayManager=&pool;int width=16<<(c%7),height=16<<((c/7)%6);textures.push_back({width,height});GenBitmap bitmap{c%5==0?-1:112};KOp parent,linked;linked.Cache=&bitmap;parent.Link=&linked;sViewport master{0,-1,{7,11,647+c,371+c}};viewport(master);word(bitmap.Texture);word(width);word(height);pool.SetMasterViewport(master);int src=c%14;word(src);pool.LastOutput=&pool.RT[0];requestedInput=src<13?&pool.RT[src]:nullptr;clearCalls();Exec_IPP_RenderTarget(&parent,&env);draw();viewport(pool.Master);word(pool.LastOutput?unsigned(pool.LastOutput-pool.RT):~0u);for(auto&t:pool.RT){word(t.Refs);if(t.Size<4){real(t.ScaleUV[0]);real(t.ScaleUV[1]);}}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'target.cpp';exe=pathlib.Path(tmp)/'target';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/overlay-render-target.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 256 original linked render-target executions')
