#!/usr/bin/env python3
"""Original intro-build MakeProjectionMatrix, including half-pixel offsets."""
import pathlib,subprocess,tempfile
import verify_frame as reference
root=reference.root;oracle=reference.oracle;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_start.cpp').read_text(encoding='cp1252');function=oracle.function('void sMaterialEnv::MakeProjectionMatrix(')
support=reference.support[:reference.support.index('struct KEnvironment{')]+r'''
#define sINTRO 1
#define sMEO_NORMALISED 1
#define sMEO_PERSPECTIVE 0
struct System{int ViewportX,ViewportY;}systemObject,*sSystem=&systemObject;
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void matrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})for(float f:{v.x,v.y,v.z,v.w})real(f);}
'''
driver=r'''
int main(){word(512);for(int c=0;c<512;c++){sMaterialEnv e;e.Init();e.Orthogonal=c%2;e.ZoomX=.5f+c*.001f;e.ZoomY=.75f+c*.002f;e.CenterX=float(c%11-5)*.125f;e.CenterY=float(c%7-3)*.0625f;e.NearClip=.0625f+c*.001f;e.FarClip=c%19==0?0.f:float(100+c);systemObject.ViewportX=c%23==0?0:320+c;systemObject.ViewportY=c%29==0?0:200+c;word(e.Orthogonal);for(float f:{e.ZoomX,e.ZoomY,e.CenterX,e.CenterY,e.NearClip,e.FarClip})real(f);word(systemObject.ViewportX);word(systemObject.ViewportY);sMatrix result;e.MakeProjectionMatrix(result);matrix(result);}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'projection.cpp';exe=pathlib.Path(tmp)/'projection';cpp.write_text(support+reference.camera+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/projection.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 512 original projection matrices')
