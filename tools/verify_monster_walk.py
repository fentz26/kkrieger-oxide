#!/usr/bin/env python3
"""Original KKMonsterWalk, real collision rays and original spline evaluator."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_player_motion as motion
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
source=(base/'kkriegergame.cpp').read_text(encoding='cp1252');oracle.source=source
walk=oracle.function('void KKMonsterWalk(')
a=source.index('struct KKMonsterWalkInfo');b=source.index('void KKMonsterWalk(',a);definitions=source[a:b]
header=(base/'kkriegergame.hpp').read_text(encoding='cp1252');a=header.index('struct KKMonsterWalkMem ');b=header.index('\n};',a)+3;memory=header[a:b]
oracle.source=(base/'kdoc.cpp').read_text(encoding='cp1252');splines='\n'.join(oracle.function(s) for s in ['sF32 KSplineChannel::Eval(','void KSpline::Eval('])
support=motion.support.replace('struct sVector {','struct sVector {\n void Sub3(const sVector&a){x-=a.x;y-=a.y;z-=a.z;}void Add3(const sVector&a,const sVector&b){x=a.x+b.x;y=a.y+b.y;z=a.z+b.z;}\n void Lin3(const sVector&a,const sVector&b,float t){x=a.x+(b.x-a.x)*t;y=a.y+(b.y-a.y)*t;z=a.z+(b.z-a.z)*t;}\n void Cross4(const sVector&a,const sVector&b){Cross3(a,b);w=0;}\n void Rotate3(const sMatrix&m,const sVector&v);void Rotate4(const sMatrix&m,const sVector&v);void Rotate34(const sMatrix&m){sVector v=*this;Rotate34(m,v);}\n')
support=support.replace('struct sVector {','struct sMatrix;\nstruct sVector {',1)
support=support.replace('struct KKriegerGame {','struct KKriegerGame {int CollisionForMonsterMode=0;')
support+=r'''
struct sF323{float x,y,z;};
#define sPI 3.1415926535897932384626433832795
#define KSI_CUBIC 0
#define KSI_LINEAR 1
#define KSI_STEP 2
int sFtol(float v){return int(std::nearbyint(v));}double sFSin(double v){return sin(v);}template<class T>T sRange(T v,T hi,T lo){return v>=hi?hi:v<=lo?lo:v;}
void sVector::Rotate3(const sMatrix&m,const sVector&v){Scale3(m.i,v.x);AddScale3(m.j,v.y);AddScale3(m.k,v.z);}
void sVector::Rotate4(const sMatrix&m,const sVector&v){Scale4(m.i,v.x);AddScale4(m.j,v.y);AddScale4(m.k,v.z);AddScale4(m.l,v.w);}
struct KSplineKey{float Time,Value;};struct KSplineChannel{int KeyCount=0;KSplineKey*Keys=nullptr;float Eval(float,int);};struct KSpline{KSplineChannel Channel[4];int Count=0,Interpolation=0;void Eval(float,sVector&);};
struct KEnvironment{KKriegerGame*Game;int CurrentTime=0,TimeReset=0;};
'''
driver=r'''
void inputcell(KKriegerCell&cell,int id,int mode,float x,float y,float z,float sx,float sy,float sz){sVector v[8];for(int k=0;k<8;k++)v[k]={x-((k&1)-0.5f)*sx,y-(((k>>1)&1)-0.5f)*sy,z-(((k>>2)&1)-0.5f)*sz,1};word(mode);for(auto p:v)vector(p);cell.Init(v,mode);cell.Id=id;}
void matrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vector(v);}
void dump(KKMonsterWalkMem&m){word(m.Reset);for(auto cell:m.FootCell)word(cell?cell->Id:~0u);for(auto v:m.FootOld)vector(v);for(auto v:m.FootNew)vector(v);for(auto t:m.StepTime)word(t);word(m.LastTime);vector(m.OldPos);vector(m.NewPos);vector(m.FootCenter);word(m.State);word(m.LastLeg);word(m.NextLeg);word(m.Idle);vector(m.Deviation);word(m.DeviationFlag);word(m.InitSteps);for(auto v:m.FootDelta)vector(v);for(auto v:m.LegPosOut)vector(v);real(m.LegTimeOut);}
int main(){word(80);word(32);for(int c=0;c<80;c++){
 KKriegerGame game;KEnvironment env{&game};KKriegerCellAdd adds[2]{};KKriegerCell other[2]{};KKriegerCellDynamic dynamic{};word(4);word(1);
 inputcell(adds[0],0,0,0,0,0,10,10,10);inputcell(adds[1],1,0,8,0,0,10,10,10);inputcell(other[0],2,1,3,-2,0,20,1,20);inputcell(other[1],3,2,3,0,0,1,4,4);inputcell(dynamic,4,1,c%7,-0.75f,0,0.5f,0.5f,2);
 for(auto&cell:adds){*game.CellAdd.Add()=&cell;*game.CellList.Add()=&cell;}for(auto&cell:other)*game.CellList.Add()=&cell;game.DCellUsed=1;game.DCellPtr[0]=&dynamic;game.CellConnect();
 KKMonsterWalkMem mem{};mem.Reset=1;KKMonsterWalkInfo info=KKMonsterWalkInfoTable[c%5];info.Flags=c/5;word(c%5);word(info.Flags);word(c%2);
 KSpline curve;curve.Count=4;curve.Interpolation=c%3;KSplineKey keys[4][3]={{{0,0},{0.5f,0.125f},{1,0}},{{0,0},{0.5f,0.25f},{1,0}},{{0,0},{0.5f,-0.0625f},{1,0}},{{0,0},{0.5f,1},{1,2}}};for(int i=0;i<4;i++)curve.Channel[i]={3,keys[i]};
 for(int k=0;k<32;k++){
  env.CurrentTime=k*90+(k>20?-300:0);env.TimeReset=k==14 && c%4==0;int cellid=c%11==0?-1:k%2;
  sMatrix mat;mat.InitEuler(0,(c%5-2)*0.125f+k*0.015625f,0);mat.l={k*0.25f-1.f,0.75f,(k%5-2)*0.125f,1};
  if(k==8){mem.Deviation={0.25f,0,-0.125f,0};mem.DeviationFlag=3;}
  word(env.CurrentTime);word(env.TimeReset);word(cellid);matrix(mat);vector(mem.Deviation);word(mem.DeviationFlag);
  KKMonsterWalk(&env,&mem,&info,c%2?&curve:nullptr,mat,cellid<0?nullptr:&adds[cellid]);matrix(mat);dump(mem);word(game.CollisionForMonsterMode);
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'walk.cpp';exe=pathlib.Path(tmp)/'walk';cpp.write_text(support+memory+definitions+motion.functions+splines+walk+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/monster-walk.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 2560 original monster walk updates')
