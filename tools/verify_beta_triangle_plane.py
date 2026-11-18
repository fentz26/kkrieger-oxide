#!/usr/bin/env python3
"""Execute released collision triangle-plane constructor and all math callees."""
import runpy,pathlib,struct,random,hashlib
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
from unicorn.x86_const import UC_X86_REG_EAX
rng=random.Random(0x200430);out=bytearray(pack(256));points=0x3004000;result=0x3004100
for case in range(256):
 vertices=[[rng.uniform(-20.,20.) for _ in range(3)]+[1.] for _ in range(3)]
 if case<16:
  scale=[0.,1e-8,1e-5,1e-3][case%4];vertices=[[0.,0.,0.,1.],[scale,0.,0.,1.],[0.,scale,0.,1.]]
 if case%17==0:vertices[2]=vertices[1][:]
 data=struct.pack('<12f',*[v for p in vertices for v in p]);u.mem_write(points,data);u.mem_write(result,bytes([0xa5])*16)
 u.mem_write(stack,pack(0x5000000,result,points,points+16,points+32));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81a75b,0x5000000,count=5000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+20
 hit=u.reg_read(UC_X86_REG_EAX)&255;assert hit in [0,1]
 out.extend(data);out.extend(pack(hit));out.extend(u.mem_read(result,16))
p=root/'tests/fixtures/beta-triangle-plane.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
