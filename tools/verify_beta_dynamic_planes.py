#!/usr/bin/env python3
"""Execute actual dynamic ApplyMatrix, including all math callees."""
import runpy,pathlib,struct,random,hashlib
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
rng=random.Random(0x200425);out=bytearray(pack(128));dynamic=0x3004000;matrix=0x3006000
for case in range(128):
 count=case%7
 planes=[]
 for i in range(6):
  n=[rng.uniform(-1.,1.) for _ in range(3)];length=sum(x*x for x in n)**0.5
  planes.extend([v/length for v in n]+[rng.uniform(-4.,4.)])
 m=[rng.uniform(-2.,2.) for _ in range(16)];m[3]=m[7]=m[11]=0.;m[15]=1.
 if case<7:m=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,3.,-2.,1.,1.]
 if case in [7,8,9,10]:
  scales=[[-1.,2.,0.5],[1e-20,1e-20,1e-20],[1e20,1e20,1e20],[0.125,8.,-2.]][case-7]
  m=[scales[0],0.,0.,0.,0.,scales[1],0.,0.,0.,0.,scales[2],0.,3.,-2.,1.,1.]
 geometry=struct.pack('<24f',*planes);transform=struct.pack('<16f',*m)
 u.mem_write(dynamic,bytes([0xa5])*0x2a4);u.mem_write(dynamic+0xf0,pack(count));u.mem_write(dynamic+0x13c,geometry);u.mem_write(matrix,transform)
 u.mem_write(stack,pack(0x5000000,matrix));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,dynamic);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81a880,0x5000000,count=30000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+8 and u.mem_read(dynamic+0x138,4)==pack(0)
 out.extend(pack(count));out.extend(geometry);out.extend(transform);out.extend(u.mem_read(dynamic+32,count*16));out.extend(u.mem_read(dynamic+0xe0,16));out.extend(u.mem_read(dynamic+0x1e4,64))
p=root/'tests/fixtures/beta-dynamic-planes.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
