#!/usr/bin/env python3
"""Original sINTRO Scene_Walk persistent state and child-visible variables."""
import pathlib,subprocess,tempfile
import verify_scene_limb as reference
root=reference.root;oracle=reference.oracle
oracle.source=(root/'.reference/fr_public/werkkzeug3_kkrieger/genscene.cpp').read_text(encoding='cp1252')
execute=oracle.function('void __stdcall Exec_Scene_Walk(')
# The x86 stdcall signature stores eight sF323 arguments contiguously. ARM64
# HFA arguments do not have that layout; pack only this ABI-dependent access.
execute=execute.replace('  sMatrix mat;//,save;', '  sF323 packedFeet[]={l0,l1,l2,l3,l4,l5,l6,l7};\n  sMatrix mat;//,save;').replace('(&l0)[i]', 'packedFeet[i]')
support=reference.support[:reference.support.index('struct KOp')]
support=support.replace('#define __stdcall','#define sINTRO 1\n#define sZONE(x)\n#define sREGZONE(x)\n#define KV_LEG_TIMES 27\n#define KV_LEG_L0 16\n#define __stdcall')
support=support.replace('void Init(float a,float b,float c,float d)', 'void Init(float a=0,float b=0,float c=0,float d=1)')
support=support.replace('void Rotate3(const sMatrix&);','''void Rotate3(const sMatrix&);void Rotate3(const sMatrix&m,const sVector&v){*this=v;Rotate3(m);}void Sub3(const sVector&v){x-=v.x;y-=v.y;z-=v.z;}void Sub3(const sVector&a,const sVector&b){x=a.x-b.x;y=a.y-b.y;z=a.z-b.z;}void Add3(const sVector&a,const sVector&b){x=a.x+b.x;y=a.y+b.y;z=a.z+b.z;}void Scale3(float s){x*=s;y*=s;z*=s;}void Lin3(const sVector&a,const sVector&b,float t){x=a.x+(b.x-a.x)*t;y=a.y+(b.y-a.y)*t;z=a.z+(b.z-a.z)*t;}void Rotate34(const sMatrix&);void Rotate34(const sMatrix&m,const sVector&v){*this=v;Rotate34(m);}''')
support+='''
void sVector::Rotate34(const sMatrix&m){Rotate3(m);Add3(m.l);w=1;}
struct sInt2{int x,y;};struct sF322{float x,y;};
struct WalkMem {int Reset=1;sVector FootOld[8],FootNew[8];int StepTime[8]{},LastTime=0;sVector OldPos,NewPos,FootCenter;int State=0,LastLeg=0,NextLeg=0,Idle=0;sVector Deviation;int DeviationFlag=0,InitSteps=0;sVector FootDelta[8];};static WalkMem*mem;
struct KSpline{void Eval(float t,sVector&v){v={0,t*.5f,0,t};}};
struct KEnvironment;
struct KOp{void ExecInputs(KEnvironment*);};
struct KEnvironment{WalkMem memory;int CurrentTime=0,TimeReset=0;sVector Var[32];struct{sMatrix stack[8];int index=0;sMatrix&Top(){return stack[index];}void Push(sMatrix m){stack[++index]=m;}void Pop(){index--;}}ExecStack;sMatrix drawn;sVector variables[32];template<class T>T*GetInst(KOp*){return &memory;}};
void KOp::ExecInputs(KEnvironment*e){e->drawn=e->ExecStack.Top();for(int i=0;i<32;i++)e->variables[i]=e->Var[i];}
int sFtol(float f){return int(nearbyintf(f));}template<class T>T sMin(T a,T b){return a<b?a:b;}template<class T>T sRange(T v,T maximum,T minimum){return v<minimum?minimum:v>maximum?maximum:v;}double sFSin(double v){return sin(v);}const double sPI=3.14159265358979323846;
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vec(sVector v){for(float f:{v.x,v.y,v.z,v.w})real(f);}void matrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vec(v);}
'''
driver=r'''
int main(){word(32);word(32);KOp op;KSpline spline;
for(int c=0;c<32;c++){KEnvironment e;int count=c%3==0?8:c%2?4:2;unsigned flags=(c%4)*4|(c%5==0?1:0)|(c%7==0?0x100:0);int hasSpline=c%3==0,runGroup=count>=4 && c%2?4:2;word(count);word(flags);word(hasSpline);word(runGroup);float ss0=.25f+(c%3)*.0005f;for(float f:{.25f,.5f,ss0,.1245f,.1255f,.0635f})real(f);
sF323 feet[8]={{-.5f,0,-.5f},{.5f,0,-.5f},{-.5f,0,.5f},{.5f,0,.5f},{-.5f,0,-1},{.5f,0,-1},{-.5f,0,1},{.5f,0,1}};
for(auto p:feet)for(float f:{p.x,p.y,p.z})real(f);
for(int k=0;k<32;k++){e.CurrentTime=k*37;if(k>=24)e.CurrentTime-=1000;e.TimeReset=k==24;e.ExecStack.stack[0].Init();e.ExecStack.stack[0].l={float(k)*.125f,0,float(k%7)*.25f,1};for(int i=0;i<32;i++)e.Var[i]={float(i),float(i+1),float(i+2),float(i+3)};
word(e.CurrentTime);word(e.TimeReset);matrix(e.ExecStack.Top());
Exec_Scene_Walk(&op,&e,flags,count,.1f,.3f,.75f,{2,runGroup},{.25f,.5f},{ss0,.1245f},{.1255f,.0635f},feet[0],feet[1],feet[2],feet[3],feet[4],feet[5],feet[6],feet[7],1,2,hasSpline?&spline:nullptr);e.memory.Reset=0;
matrix(e.drawn);vec(e.variables[KV_LEG_TIMES]);for(int i=0;i<count;i++)vec(e.variables[KV_LEG_L0+i]);auto&m=e.memory;for(int n:{m.State,m.LastLeg,m.NextLeg,m.Idle,m.LastTime,m.InitSteps})word(n);vec(m.NewPos);vec(m.FootCenter);for(int i=0;i<count;i++){vec(m.FootOld[i]);vec(m.FootNew[i]);vec(m.FootDelta[i]);word(m.StepTime[i]);}for(int i:{KV_LEG_TIMES,KV_LEG_L0})vec(e.Var[i]);word(e.ExecStack.index);
}}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'walk.cpp';exe=pathlib.Path(tmp)/'walk';cpp.write_text(support+execute+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/scene-walk.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 1024 original persistent walker executions')
