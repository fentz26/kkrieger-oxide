#!/usr/bin/env python3
"""Trace unchanged RenderPaintJobs and Material11Insert with instrumented sinks."""
import pathlib
import subprocess
import tempfile
import verify_mesh as oracle
root = oracle.root
oracle.source = (root/'.reference/fr_public/werkkzeug3_kkrieger/engine.cpp').read_text(encoding='cp1252')
render = oracle.function('void Engine_::RenderPaintJobs(')
source = (root/'.reference/fr_public/werkkzeug3_kkrieger/genmaterial.cpp').read_text(encoding='cp1252')
insert = source[source.index('class Material11Insert'):source.index('class Material20Insert')]
support = r'''
#include <cstdio>
#include <cstring>
#include <vector>
#include <algorithm>
using sInt=int;using sU32=unsigned;
#define sINTRO 1
#define sZONE(x)
#define ENGU_BASE 0
#define ENGU_SHADOW 3
#define ENGU_LIGHT 4
#define ENGU_POSTLIGHT 5
#define sPS_11 1
#define GENOVER_CLRDESTALPHA 3
#define GENOVER_ADDDESTALPHA 2
#define sVCF_STENCIL 4
#define sCopyMem std::memcpy
struct Event{int op,arg;};std::vector<Event>trace;
void emit(int op,int arg=0){trace.push_back({op,arg});}
void word(unsigned v){std::fwrite(&v,4,1,stdout);}
struct sMatrix{void TransR(){}};
struct sVector{void InitColor(int){}void Rotate34(const sMatrix&,const sVector&) {}};
struct Rect{int id;};
struct System{void SetScissor(Rect*r){if(r)emit(7,r->id);else emit(0);}void Clear(int){emit(4);}}system_;System*sSystem=&system_;
struct Overlay{int CurrentShader;void FXQuad(int n){emit(n==3?1:2);}}overlay;Overlay*GenOverlayManager=&overlay;
struct sMaterial11{static void SetShadowStates(int){emit(3);}};
struct EngLight{sVector Position;int Color;float Range,Amplify;int Id;Rect LightRect;};
struct EngMaterialInsert{virtual int GetPriority()=0;virtual void BeforeUsage(int,int,const EngLight*)=0;virtual void AfterUsage(int,int,const EngLight*)=0;};
struct EnvType{sMatrix ModelSpace;sVector LightPos,LightColor;float LightRange,LightAmplify;};
struct Material{void Set(const EnvType&) {}};
struct GenMaterialPass{int Usage;Material*Mtrl;};
struct EngPaintInfo{sVector LightPos;float LightRange;int LightId,StencilFlags;void*BoneData;};
struct Mesh{void PaintJob(int id,GenMaterialPass*,EngPaintInfo&){emit(8,id);}};
struct Stack{void Push(sMatrix&){}void Pop(){}};
struct KEnvironment{sVector Var[1];Stack ExecStack;};
struct Op{int id;void*SceneMemLink;void ExecWithNewMem(KEnvironment*,void**){emit(8,id);}};
struct EffectJob{Op*Op;int VarStart,VarCount;sVector*Animation;sMatrix Matrix;};
struct PaintJob{unsigned SortKey;int JobId;EngLight*Light;GenMaterialPass*Pass;int MatrixId;Mesh*Mesh;int Flags;void*BoneData;EffectJob*EffJob;};
struct Jobs{int Count;PaintJob**Array;PaintJob*operator[](int i){return Array[i];}};
struct Engine_{Jobs PaintJobs;EnvType Env;sMatrix Matrices[1];EngMaterialInsert*Inserts[256];bool NeedCurrentRender[256];void ApplyViewProject(){emit(5);}void RenderPaintJobs(KEnvironment*);};
Engine_*Engine;
struct Targets{void GrabToTarget(unsigned){emit(6);}}targets;Targets*RenderTargetManager=&targets;
'''
driver = r'''
int main(){word(128);for(int c=0;c<128;c++){
Engine_ engine{};Engine=&engine;Material11Insert insert;overlay.CurrentShader=c%2;
for(int i=0;i<256;i++){engine.Inserts[i]=(i+c)%3==0?&insert:nullptr;engine.NeedCurrentRender[i]=(i+c)%4==0;}
int n=c%17;word(overlay.CurrentShader);word(n);
EngLight lights[3]{};for(int i=0;i<3;i++){lights[i].Id=i;lights[i].LightRect.id=i;}
std::vector<PaintJob>jobs(n);std::vector<PaintJob*>pointers;std::vector<Op>ops(n);std::vector<EffectJob>effects(n);std::vector<GenMaterialPass>passes(n);Material material;Mesh mesh;
for(int i=0;i<n;i++){
int pass=(i*3+c)%4,usage=(i+c)%8;bool effect=(i+c)%5==0;int light=(!effect&&(usage==3||usage==4))?(i+c)%3:-1;
auto&job=jobs[i];job.SortKey=(pass<<24)|((light<0?0:light)<<20)|(usage<<16)|(i*7);job.JobId=effect?-1:i;job.Light=light<0?nullptr:&lights[light];passes[i]={usage,&material};job.Pass=&passes[i];job.Mesh=&mesh;ops[i].id=i;effects[i]={&ops[i],0,0,nullptr,{}};job.EffJob=&effects[i];pointers.push_back(&job);
}
std::stable_sort(pointers.begin(),pointers.end(),[](auto*a,auto*b){return a->SortKey<b->SortKey;});
// IDs in the instrumented sink denote sorted plan indices.
for(int i=0;i<n;i++){auto*j=pointers[i];if(j->JobId>=0)j->JobId=i;else j->EffJob->Op->id=i;word(j->SortKey);word(j->JobId<0);word(j->Light?j->Light->Id:~0u);}
for(int i=0;i<4;i++){word(engine.Inserts[i]!=nullptr);word(engine.NeedCurrentRender[i]);}
engine.PaintJobs={n,pointers.data()};KEnvironment env;engine.RenderPaintJobs(&env);
word(trace.size());for(auto&e:trace){word(e.op);word(e.arg);}trace.clear();}}
'''
with tempfile.TemporaryDirectory() as tmp:
    cpp=pathlib.Path(tmp)/'render.cpp';exe=pathlib.Path(tmp)/'render'
    cpp.write_text(support+insert+render+driver)
    subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
    data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/paint-render.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 128 original render/hook traces')
