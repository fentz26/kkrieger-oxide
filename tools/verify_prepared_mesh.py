#!/usr/bin/env python3
"""Original EngMesh CPU vertex/job buffers; no GPU code in the runtime oracle."""
import pathlib,subprocess,tempfile
import verify_cube as cube
import verify_mesh as oracle
root=oracle.root
engine=(root/'.reference/fr_public/werkkzeug3_kkrieger/engine.cpp').read_text()
def extract(signature):
 old=oracle.source;oracle.source=engine
 try:return oracle.function(signature)
 finally:oracle.source=old
support=cube.support.replace('void CalcNormals(int,int);','void CalcNormals(int,int);void CalcNormal(sVector&,int,int,int);void CalcFaceNormalAccurate(sVector&,int);void CalcFacePlane(sVector&,int);')
support=support.replace('void Init(float a,float b,float c,float d)', 'float operator[](int i)const{switch(i){case 0:return x;case 1:return y;case 2:return z;default:return w;}}\n void Init(float a,float b,float c,float d)')
support+='''
using sU16=uint16_t;
#define MPP_STATIC 1
#define MPP_SHADOW 2
#define MPP_SPRITES 3
#define MPP_THICKLINES 4
float sFInvSqrt(float v){return 1.0f/sqrtf(v);}
int sFtol(float v){return (int)floorf(v);}
template<class T>T sRange(T v,T max,T min){return std::min(max,std::max(min,v));}
template<class T>T sMin(T a,T b){return std::min(a,b);}template<class T>T sMax(T a,T b){return std::max(a,b);}
struct sAABox{sVector Min,Max;};
struct sVertexTSpace3Big{float x,y,z,nx,ny,nz,sx,sy,sz;unsigned c;float u,v;};
struct EngMesh {
 struct SilEdge{sU16 Vert[2];int Face[2];};
 struct Job {int MtrlId,Program; sAABox BBox;int VertexCount=0,IndexCount=0,EdgeCount=0,PlaneCount=0,PartCount=0;
 int *VertexBuffer,*IndexBuffer,*FaceMap,*FacePartEnd,*EdgePartEnd;SilEdge*Edges;sVector*Planes=nullptr;
 void Alloc(int vc,int ic,int ec,int fmc,int pc){VertexCount=vc;IndexCount=ic;EdgeCount=ec;PartCount=pc;VertexBuffer=new int[vc];IndexBuffer=new int[ic];FaceMap=new int[fmc];Edges=new SilEdge[ec];FacePartEnd=new int[pc];EdgePartEnd=new int[pc];std::fill(FacePartEnd,FacePartEnd+pc,-1);std::fill(EdgePartEnd,EdgePartEnd+pc,-1);}
 };
 int VertCount=0,PartCount=0;sVertexTSpace3Big*Vert=nullptr;float*PartBBs=nullptr;
 void FillVertexBuffer(GenMesh*);void CalcPartBoundingBoxes(GenMesh*);void CalcBoundingBox(GenMesh*,int,sAABox&);void PrepareJob(Job*,GenMesh*);void PrepareSpriteJob(Job*,GenMesh*);void PrepareThickLinesJob(Job*,GenMesh*);
};
'''
functions=cube.functions+'\n'+'\n'.join(oracle.function(s) for s in ['void GenMesh::CalcNormal(','void GenMesh::CalcFaceNormalAccurate(','void GenMesh::CalcFacePlane('])
functions+='\n'+'\n'.join(extract(s) for s in ['static sInt ConvertRGB(','static sU32 PackColor(','void EngMesh::FillVertexBuffer(','void EngMesh::CalcPartBoundingBoxes(','void EngMesh::CalcBoundingBox(','void EngMesh::PrepareJob(','void EngMesh::PrepareSpriteJob(','void EngMesh::PrepareThickLinesJob('])
driver=r'''
int main(){word(64);for(int c=0;c<64;c++){
 sFSRT srt={{{1,1.5f,0.75f},{0,0,0},{-0.25f,0.125f,0.5f}}};
 auto*m=Mesh_Cube(1+c%3,1+(c/3)%3,1+(c/9)%3,c%16,srt);
 for(int i=0;i<m->Face.Count;i++){m->Face[i].Material=(i+c)%3;m->Face[i].Used=(i+c)%4!=0;}
 for(int i=0;i<m->Vert.Count;i++)m->VertBuf[i*5+3].Init((i%7)*0.25f-0.25f,(i%5)*0.375f,(i%3)*0.5f,0.75f);
 m->Parts.Count=0;*m->Parts.Add()=0;if(c%2)*m->Parts.Add()=m->Face.Count/2;
 m->CalcNormals(0,7);EngMesh eng;eng.FillVertexBuffer(m);eng.CalcPartBoundingBoxes(m);
 word(eng.VertCount);for(int i=0;i<eng.VertCount;i++){auto&v=eng.Vert[i];for(float f:{v.x,v.y,v.z,v.nx,v.ny,v.nz,v.sx,v.sy,v.sz})real(f);word(v.c);real(v.u);real(v.v);}
 word(eng.PartCount);for(int i=0;i<eng.PartCount*6;i++)real(eng.PartBBs[i]);
 word(8);for(int material=1;material<=2;material++)for(int program=1;program<=4;program++){
 EngMesh::Job job;job.MtrlId=material;job.Program=program;eng.PrepareJob(&job,m);
 word(material);word(program);for(int a=0;a<3;a++)real(job.BBox.Min[a]);for(int a=0;a<3;a++)real(job.BBox.Max[a]);
 word(job.VertexCount);for(int i=0;i<job.VertexCount;i++)word(job.VertexBuffer[i]);word(job.IndexCount);for(int i=0;i<job.IndexCount;i++)word(job.IndexBuffer[i]);
 word(job.PlaneCount);for(int i=0;i<job.PlaneCount;i++){auto&p=job.Planes[i];for(float v:{p.x,p.y,p.z,p.w})real(v);}
 word(program==2?job.IndexCount/3:0);if(program==2)for(int i=0;i<job.IndexCount/3;i++)word(job.FaceMap[i]);
 word(job.EdgeCount);for(int i=0;i<job.EdgeCount;i++){auto&e=job.Edges[i];for(auto v:e.Vert)word(v);for(auto v:e.Face)word(v);}
 word(job.PartCount);for(int i=0;i<job.PartCount;i++){word(job.FacePartEnd[i]);word(job.EdgePartEnd[i]);}
 }
 delete m;
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'prepared.cpp';exe=pathlib.Path(tmp)/'prepared'
 cpp.write_text(support+functions+cube.cube+driver)
 subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/prepared-mesh.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes / 512 original preparation jobs')
