#!/usr/bin/env python3
"""Original shadow cache indices with persistent global face classification."""
import pathlib,subprocess,tempfile
import verify_prepared_mesh as prepared
import verify_cube as cube
import verify_mesh as oracle
root=oracle.root
support=prepared.support.replace('struct EngMesh {','''struct EngPaintInfo{sVector LightPos;float LightRange;};
#define sZONE(x)
struct EngMesh {''')
support=support.replace(' struct SilEdge{',' struct SVCache{int*IndexBuffer=nullptr;int SilIndices=0,CapIndices=0;};static unsigned char SFaceIn[65536],SPartSkip[16384];\n struct SilEdge{')
support=support.replace(' void FillVertexBuffer(GenMesh*);',' void UpdateShadowCacheJob(Job*,sVector*,SVCache*,const EngPaintInfo&);\n void FillVertexBuffer(GenMesh*);')
support+='\nunsigned char EngMesh::SFaceIn[65536]{},EngMesh::SPartSkip[16384]{};\n'
functions=prepared.functions+'\n'+prepared.extract('void EngMesh::UpdateShadowCacheJob(')
driver=r'''
int main(){word(128);for(int c=0;c<128;c++){
 sFSRT srt={{{1,1.5f,0.75f},{0,0,0},{-0.25f,0.125f,0.5f}}};auto*m=Mesh_Cube(1+c%3,1+(c/3)%3,1+(c/9)%3,c%16,srt);
 for(int i=0;i<m->Face.Count;i++){m->Face[i].Material=(i+c)%3;m->Face[i].Used=(i+c)%4!=0;}
 EngMesh eng;eng.PartCount=m->Parts.Count;EngMesh::Job job;job.MtrlId=1+c%2;job.Program=MPP_SHADOW;eng.PrepareJob(&job,m);
 EngPaintInfo info;info.LightPos.Init((c%9-4)*0.375f,((c/9)%7-3)*0.25f,((c/7)%5-2)*0.5f,1);info.LightRange=c%4;
 EngMesh::SVCache cache;eng.UpdateShadowCacheJob(&job,job.Planes,&cache,info);
 real(info.LightPos.x);real(info.LightPos.y);real(info.LightPos.z);word(job.VertexCount*2);
 for(int i=0;i<job.VertexCount;i++){auto&p=m->VertPos(job.VertexBuffer[i]);for(float w:{1.0f,0.0f}){real(p.x);real(p.y);real(p.z);real(w);}}
 word(cache.SilIndices);word(cache.CapIndices);for(int i=0;i<cache.SilIndices+cache.CapIndices;i++)word(cache.IndexBuffer[i]);
 delete[]cache.IndexBuffer;delete m;
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'shadow.cpp';exe=pathlib.Path(tmp)/'shadow'
 cpp.write_text(support+functions+cube.cube+driver)
 subprocess.run(['clang++','-std=c++17','-Wno-register','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True)
 data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/shadow.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes / 128 original shadow volumes')
