#!/usr/bin/env python3
"""Original enabled portal-visibility branch and portal-box transforms."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger'
engine=(base/'engine.cpp').read_text()
def extract(text,sig):
 old=oracle.source;oracle.source=text
 try:return oracle.function(sig)
 finally:oracle.source=old
support=r'''
#include <cstdint>
#include <cstdio>
#include <algorithm>
using sInt=int;using sU8=unsigned char;using sU32=unsigned;using sF32=float;using sBool=bool;
#define sTRUE true
#define sFALSE false
#define sINTRO 0
#define sZONE(x)
template<class T>T sMin(T a,T b){return a<b?a:b;}template<class T>T sMax(T a,T b){return a>b?a:b;}
struct sMatrix;
struct sVector {float x=0,y=0,z=0,w=0;void Init(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}
 void Scale4(const sVector&a,float f){x=a.x*f;y=a.y*f;z=a.z*f;w=a.w*f;}
 void AddScale4(const sVector&a,float f){x+=a.x*f;y+=a.y*f;z+=a.z*f;w+=a.w*f;}
 void Add4(const sVector&a){x+=a.x;y+=a.y;z+=a.z;w+=a.w;}
 void Rotate34(const sMatrix&,const sVector&);void Rotate4(const sMatrix&,const sVector&);void Lin4(const sVector&,const sVector&,float);
};
struct sMatrix {sVector i,j,k,l;void Init(){i.Init(1,0,0,0);j.Init(0,1,0,0);k.Init(0,0,1,0);l.Init(0,0,0,1);}};
struct sFRect {float x0,y0,x1,y1;void Init(float a,float b,float c,float d){x0=a;y0=b;x1=c;y1=d;}void And(const sFRect&);void Or(const sFRect&);float XSize(){return x1-x0;}float YSize(){return y1-y0;}};
struct sAABox {sVector Min,Max;};
struct GenScene {int Id;GenScene*Next;bool SectorPaint=false;int SectorVisited=0;sFRect PortalBox;};
struct KKriegerCell {GenScene*Scene;};struct KEnvironment{};
struct Alloc {template<class T>T*Alloc(){return new T;}};
struct Engine_ {struct PortalJob {PortalJob*Next;GenScene*Sectors[3];int Cost;sVector PCube[8];};
 struct MemType {template<class T>T*Alloc(){return new T;}}Mem;
 PortalJob*PortalJobs=nullptr;GenScene*SectorJobs=nullptr;sMatrix ViewProject;
 void AddPortalJob(GenScene*[],const sAABox&,const sMatrix&,int);void ProcessPortals(KEnvironment*,KKriegerCell*);void PortalVisR(GenScene*,int,const sFRect&);
};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vector(sVector v){for(float f:{v.x,v.y,v.z,v.w})real(f);}void matrix(sMatrix m){for(auto v:{m.i,m.j,m.k,m.l})vector(v);}
'''
# Drop an unused wrapper whose name collides with its template allocator method.
support=support.replace('struct Alloc {template<class T>T*Alloc(){return new T;}};','')
types=(base/'_types.cpp').read_text(encoding='cp1252')
functions='\n'.join(extract(types,s) for s in ['void sVector::Rotate34(const sMatrix &m,const sVector &v)','void sVector::Rotate4(const sMatrix &m,const sVector &v)','void sVector::Lin4(','void sFRect::And(','void sFRect::Or('])
functions+='\n'+'\n'.join(extract(engine,s) for s in ['void Engine_::AddPortalJob(','static void ProjectAndInclude(','void Engine_::PortalVisR('])
process=extract(engine,'void Engine_::ProcessPortals(')
# Keep the original visibility phase; execution/frustum setup is tested separately.
functions+='\n'+process[:process.index('  // exec phase')]+'\n#endif\n}\n'
driver=r'''
int main(){word(128);for(int c=0;c<128;c++){
 Engine_ engine;engine.ViewProject.Init();if(c&1){engine.ViewProject.k.w=1;engine.ViewProject.l.w=0;engine.ViewProject.l.z=-0.5f;}
 GenScene sectors[6];for(int i=0;i<6;i++){sectors[i].Id=i;sectors[i].Next=engine.SectorJobs;engine.SectorJobs=&sectors[i];sectors[i].PortalBox.Init(1e15f,1e15f,-1e15f,-1e15f);}
 KKriegerCell observer{&sectors[c%6]};bool all=c%7==6;matrix(engine.ViewProject);word(all?~0u:c%6);word(6);word(6);
 const int links[6][2]={{0,1},{1,2},{2,3},{1,3},{3,4},{4,0}};
 for(int k=0;k<6;k++){
  int middle=k%3==0?5:-1;int cost=k==c%6?256:((k+c)%3)*64;
  sAABox box;box.Min.Init(-0.5f+k*0.25f,-0.25f,(c%7-3)*0.5f,1);box.Max.Init(box.Min.x+0.75f,0.75f,box.Min.z+0.5f,1);
  sMatrix transform;transform.Init();transform.i.x=1+(c%3)*0.25f;transform.j.y=0.75f;transform.k.z=1.25f;transform.i.y=0.25f;transform.j.z=-0.125f;transform.k.x=0.125f;transform.l.x=(k%2-0.5f)*0.75f;transform.l.z=(c%5)*0.25f;
  word(links[k][0]);word(links[k][1]);word(middle);word(cost);for(float f:{box.Min.x,box.Min.y,box.Min.z,box.Max.x,box.Max.y,box.Max.z})real(f);matrix(transform);
  GenScene*adjacent[3]={&sectors[links[k][0]],&sectors[links[k][1]],middle>=0?&sectors[middle]:nullptr};engine.AddPortalJob(adjacent,box,transform,cost);for(auto p:engine.PortalJobs->PCube)vector(p);
 }
 engine.ProcessPortals(nullptr,all?nullptr:&observer);
 for(auto&s:sectors){word(s.SectorPaint);for(float f:{s.PortalBox.x0,s.PortalBox.y0,s.PortalBox.x1,s.PortalBox.y1})real(f);word(s.SectorVisited);}
 for(auto&s:sectors)s.SectorPaint=false;engine.ViewProject.l.x=0.125f;engine.ViewProject.i.x=0.75f;matrix(engine.ViewProject);
 engine.ProcessPortals(nullptr,all?nullptr:&observer);
 for(auto&s:sectors){word(s.SectorPaint);for(float f:{s.PortalBox.x0,s.PortalBox.y0,s.PortalBox.x1,s.PortalBox.y1})real(f);word(s.SectorVisited);}
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'visibility.cpp';exe=pathlib.Path(tmp)/'visibility'
 cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/visibility.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes / 128 original visibility graphs')
