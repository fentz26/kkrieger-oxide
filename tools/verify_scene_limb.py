#!/usr/bin/env python3
"""Original Scene_Limb handler, including parent stack and matrix variables."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
oracle.source=(base/'genscene.cpp').read_text(encoding='cp1252');execute=oracle.function('void __stdcall Exec_Scene_Limb(')
oracle.source=(base/'_types.cpp').read_text(encoding='cp1252');multiply=oracle.function('void sMatrix::MulA(')
support=r'''
#include <cstdio>
#include <cmath>
#include <vector>
using sU32=unsigned;using sInt=int;using sF32=float;
#define __stdcall
struct sF323{float x,y,z;};struct sMatrix;
struct sVector{float x=0,y=0,z=0,w=0;void Init(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}void Init4(float a,float b,float c,float d){Init(a,b,c,d);}float Dot3(const sVector&v)const{return x*v.x+y*v.y+z*v.z;}float Abs3(){return float(sqrt(double(Dot3(*this))));}void Unit3(){float s=float(1./sqrt(double(Dot3(*this))));x*=s;y*=s;z*=s;}void Scale4(const sVector&v,float s){x=v.x*s;y=v.y*s;z=v.z*s;w=v.w*s;}void AddScale3(const sVector&v,float s){x+=v.x*s;y+=v.y*s;z+=v.z*s;}void Cross4(const sVector&a,const sVector&b){x=a.y*b.z-a.z*b.y;y=a.z*b.x-a.x*b.z;z=a.x*b.y-a.y*b.x;w=0;}void Add3(const sVector&v){x+=v.x;y+=v.y;z+=v.z;}void Rotate3(const sMatrix&);};
struct sMatrix{sVector i,j,k,l;const sVector&operator[](int n)const{return (&i)[n];}void MulA(const sMatrix&,const sMatrix&);void Init(){i={1,0,0,0};j={0,1,0,0};k={0,0,1,0};l={0,0,0,1};}};
void sVector::Rotate3(const sMatrix&m){sVector v=*this;x=m.i.x*v.x+m.j.x*v.y+m.k.x*v.z;y=m.i.y*v.x+m.j.y*v.y+m.k.y*v.z;z=m.i.z*v.x+m.j.z*v.y+m.k.z*v.z;}
struct KOp{int count;int GetInputCount(){return count;}};
struct Record{int input;sMatrix transform,other;};
struct KEnvironment{struct{sMatrix stack[8];int index=0;sMatrix&Top(){return stack[index];}void Push(sMatrix m){stack[++index]=m;}void PushMul(sMatrix m){sMatrix result;result.MulA(m,Top());Push(result);}void Pop(){index--;}}ExecStack;sMatrix variable;std::vector<Record>draws;void SetMatrix(sMatrix&m,sMatrix&save){save=variable;variable=m;}void RestoreMatrix(sMatrix&save){variable=save;}};
void ExecSceneInput(KOp*,KEnvironment*e,int input){e->draws.push_back({input,e->ExecStack.Top(),e->variable});}
float sFSqrt(float f){return float(sqrt(double(f)));}
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vec(sVector v){for(float f:{v.x,v.y,v.z,v.w})real(f);}void matrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vec(v);}
'''
driver=r'''
int main(){word(256);for(int c=0;c<256;c++){KEnvironment e;KOp op{c%7==0?c%4:2};sMatrix parent;parent.Init();parent.i={1,.25f,0,0};parent.j={0,1,.125f,0};parent.k={.125f,0,1,0};parent.l={float(c%3),float(c%5-2),float(c%7),1};e.ExecStack.stack[0]=parent;e.variable.Init();e.variable.l={17,19,23,1};
sF323 pos{float(c%9-4)*.25f,float(c%11+1)*.5f,float(c%13-6)*.125f},dir{.25f,1,.5f};if(c%31==0)dir={0,0,0};float lengths[]={.5f,1,4,-1};float a=lengths[(c/4)%4],b=lengths[(c/16)%4];unsigned flags=c%8;
word(op.count);matrix(parent);for(float f:{pos.x,pos.y,pos.z,dir.x,dir.y,dir.z,a,b})real(f);word(flags);Exec_Scene_Limb(&op,&e,pos,dir,a,b,flags);word(e.draws.size());for(auto&r:e.draws){word(r.input);matrix(r.transform);matrix(r.other);}matrix(e.variable);word(e.ExecStack.index);
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'limb.cpp';exe=pathlib.Path(tmp)/'limb';cpp.write_text(support+multiply+execute+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/scene-limb.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 256 original two-joint limb executions')
