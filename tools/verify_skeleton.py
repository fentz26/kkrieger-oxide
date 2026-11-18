#!/usr/bin/env python3
"""Original GenMinMeshAnim::EvalAnimation and matrix helpers, native oracle."""
import pathlib, subprocess, tempfile
import verify_mesh as oracle
root=oracle.root
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genminmesh.cpp').read_text(encoding='cp1252')
evaluate=oracle.function('void GenMinMeshAnim::EvalAnimation(')
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_types.cpp').read_text(encoding='cp1252')
functions='\n'.join(oracle.function(s) for s in ['void sMatrix::InitEulerPI2(', 'void sMatrix::InitSRT(', 'void sMatrix::MulA('])
support=r'''
#include <cstdio>
#include <cmath>
using sInt=int;using sF32=float;
const float sPI2F=6.2831853071795864769f;
double sFAbs(double x){return fabs(x);}void sFSinCos(float x,float&s,float&c){s=sin(double(x));c=cos(double(x));}
struct V {float x,y,z,w;void Init4(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}void Init3(float a,float b,float c){Init4(a,b,c,1);}void Scale3(float s){x*=s;y*=s;z*=s;}};
using sVector=V;
struct sMatrix {V i,j,k,l;const V&operator[](int n)const{return (&i)[n];}V&operator[](int n){return (&i)[n];}void Init(){i={1,0,0,0};j={0,1,0,0};k={0,0,1,0};l={0,0,0,1};}void InitEulerPI2(const float*);void InitSRT(const float*);void MulA(const sMatrix&,const sMatrix&);};
struct GenSpline{void Eval(float t,float original,sMatrix&m,float&dummy){m.Init();m.l={t,original,t-original,1};dummy=0;}};
struct GenMinMatrix {sMatrix BasePose;int Parent,KeyCount;float SRT[9];float*SPtr,*RPtr,*TPtr;GenSpline*Spline;sMatrix NoAnimation;float Offset,Factor;sMatrix Temp;};
struct GenMinMeshAnim {struct {int Count;GenMinMatrix*Array;GenMinMatrix&operator[](int i){return Array[i];}}Matrices;void EvalAnimation(float,float,sMatrix*);};
unsigned seed=345678;unsigned rnd(){return seed=seed*196314165u+907633515u;}float val(){return (int(rnd()%257)-128)/64.f;}
void word(unsigned x){fwrite(&x,4,1,stdout);}void real(float x){fwrite(&x,4,1,stdout);}void matrix(sMatrix&m){float*p=&m.i.x;for(int i=0;i<16;i++)real(p[i]);}
'''
driver=r'''
int main(){word(128);for(int c=0;c<128;c++){
 GenMinMatrix nodes[4]{};float keys[4][3][9];GenSpline spline;
 for(int n=0;n<4;n++){
  auto&b=nodes[n];b.Parent=n?int(rnd()%n):-1;b.BasePose.Init();b.BasePose.l={val(),val(),val(),1};b.Temp.Init();b.NoAnimation.Init();b.NoAnimation.l={val(),val(),val(),1};b.Offset=val();b.Factor=val();
  for(int i=0;i<9;i++)b.SRT[i]=val();
  int mode=(c+n)%3;b.KeyCount=mode==1?3:0;b.Spline=mode==2?&spline:nullptr;
  for(int group=0;group<3;group++){for(int i=0;i<9;i++)keys[n][group][i]=val();(&b.SPtr)[group]=((c+n+group)%4)?keys[n][group]:nullptr;}
  word(b.Parent);matrix(b.BasePose);word(mode);matrix(b.NoAnimation);real(b.Factor);real(b.Offset);
  for(float v:b.SRT)real(v);for(int group=0;group<3;group++){word((&b.SPtr)[group]!=nullptr);for(float v:keys[n][group])real(v);}
 }
 GenMinMeshAnim anim{{4,nodes}};sMatrix out[4];
 for(float time:{-1.f,0.f,0.1251f,0.75f,1.25f}){real(time);anim.EvalAnimation(time,0,out);for(int n=0;n<4;n++){matrix(nodes[n].Temp);matrix(out[n]);}}
}}
'''.replace('#PLACEHOLDER','')
# Range-for initializer_list needs its standard declaration.
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'skeleton.cpp';exe=pathlib.Path(tmp)/'skeleton'
 cpp.write_text('#include <initializer_list>\n'+support+functions+evaluate+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
p=root/'tests/fixtures/skeleton.bin';p.write_bytes(data);print(f'{p}: {len(data)} bytes')
