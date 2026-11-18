#!/usr/bin/env python3
"""Execute moving-cell projection math and actual hull entry routine."""
import runpy,pathlib,struct,random,hashlib
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_EIP
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
frame=0x3030000;particle=0x3004000;rng=random.Random(0x200427);out=bytearray(pack(128));blocked=False

def hook(u,address,size,user):
 global blocked
 if address in [0x81c612,0x81c64e]:blocked=True;u.reg_write(UC_X86_REG_EIP,0x81c67b)
u.hook_add(UC_HOOK_CODE,hook)
for case in range(128):
 planes=[[1.,0.,0.,2.],[-1.,0.,0.,2.],[0.,1.,0.,2.],[0.,-1.,0.,2.],[0.,0.,1.,2.],[0.,0.,-1.,2.]]
 p=[rng.uniform(-1.,1.) for _ in range(3)]+[1.]
 m=[rng.uniform(-1.,1.) for _ in range(16)];m[3]=m[7]=m[11]=0.;m[15]=1.
 if case%16==0:m=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.]
 data=struct.pack('<24f',*[v for plane in planes for v in plane]);matrix=struct.pack('<16f',*m);point_data=struct.pack('<4f',*p)
 u.mem_write(particle+0x40,point_data);u.mem_write(frame-4,pack(particle+0x40));u.mem_write(frame-0xe8,matrix)
 u.mem_write(frame-0x38c+32,data);u.mem_write(frame-0x38c+0xf0,pack(6))
 u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_ESP,frame-0x500);u.reg_write(UC_X86_REG_FPCW,0x143f)
 blocked=False;u.emu_start(0x81c5ba,0x81c67b,count=10000)
 result=bytes(u.mem_read(frame-0x34,12))+struct.pack('<f',1.) if not blocked else bytes(16)
 out.extend(point_data);out.extend(matrix);out.extend(pack(int(blocked)));out.extend(result)
p=root/'tests/fixtures/beta-dynamic-projection.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
