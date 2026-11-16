#!/usr/bin/env python3
"""Unchanged original Exec_Scene_Rotate, captured at its input call boundary."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genscene.cpp').read_text(encoding='cp1252');function=oracle.function('void __stdcall Exec_Scene_Rotate(')
support=r'''
#include <cstdio>
#include <cmath>
#include <vector>
using sInt=int;
#define __stdcall
struct sF323{float x,y,z;};
struct sVector{float x,y,z,w;void Init(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}void Cross4(const sVector&a,const sVector&b){x=a.y*b.z-a.z*b.y;y=a.z*b.x-a.x*b.z;z=a.x*b.y-a.y*b.x;w=0;}void Unit3(){float scale=float(1./sqrt(double(x*x+y*y+z*z)));x*=scale;y*=scale;z*=scale;}void UnitSafe3(){float e=x*x+y*y+z*z;if(e>1e-20f)Unit3();else{x=1;y=z=0;}}};
struct sMatrix{sVector i,j,k,l;};
struct Stack{std::vector<sMatrix>v;sMatrix&Top(){return v.back();}void Push(const sMatrix&m){v.push_back(m);}void Pop(){v.pop_back();}};
struct KEnvironment{Stack ExecStack;struct{sMatrix CameraSpace;}CurrentCam;};
struct KOp{sMatrix Captured;int Calls=0;void ExecInputs(KEnvironment*e){Captured=e->ExecStack.Top();Calls++;}};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void matrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})for(float f:{v.x,v.y,v.z,v.w})real(f);}
'''
driver=r'''
int main(){word(512);for(int c=0;c<512;c++){KEnvironment env;KOp op;int axis=c%7-1;sF323 dir{float(c%13-6)*.25f,float(c%11-5)*.125f,float(c%7-3)*.5f};if(c%17==0)dir={0,0,0};if(c%19==0)dir={0,1,0};sMatrix parent{{1+c*.001f,.125f,0,0},{0,1,.25f,0},{.125f,0,1,0},{float(c%5),-2,3,1}};sMatrix camera{{0,1,0,0},{-1,0,0,0},{0,0,1,0},{9,8,7,1}};env.ExecStack.Push(parent);env.CurrentCam.CameraSpace=camera;word(axis);for(float f:{dir.x,dir.y,dir.z})real(f);matrix(parent);matrix(camera);Exec_Scene_Rotate(&op,&env,dir,axis);matrix(op.Captured);word(op.Calls);matrix(env.ExecStack.Top());}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'rotate.cpp';exe=pathlib.Path(tmp)/'rotate';cpp.write_text(support+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/scene-rotate.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 512 original scene rotation calls')
