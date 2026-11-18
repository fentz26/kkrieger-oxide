#!/usr/bin/env python3
"""Execute full released cell Init(vertices,mode), including plane construction."""
import runpy,pathlib,struct,random,hashlib
from unicorn.x86_const import UC_X86_REG_EAX
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
rng=random.Random(0x200431);out=bytearray(pack(256));vertices=0x3004000;result=0x3004200
for case in range(256):
 scale=[rng.uniform(0.2,3.) for _ in range(3)];offset=[rng.uniform(-4.,4.) for _ in range(3)]
 v=[[offset[j]+scale[j]*(0.5-((i>>j)&1)) for j in range(3)]+[1.] for i in range(8)]
 if case%3==0:
  for p in v:
   for j in range(3):p[j]+=rng.uniform(-0.2,0.2)
 if case<16:
  if case%4==0:v=[v[0][:] for _ in range(8)]
  if case%4==1:
   for p in v:p[1]=0.
  if case%4==2:v[1]=v[0][:];v[3]=v[2][:]
  if case%4==3:v[0]=v[2][:]
 data=struct.pack('<32f',*[x for p in v for x in p]);mode=case%16
 u.mem_write(vertices,data);u.mem_write(result,bytes([0xa5])*0x128)
 u.mem_write(stack,pack(0x5000000,vertices,mode));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,result);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81bb6f,0x5000000,count=30000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+12
 count=struct.unpack('<I',u.mem_read(result+0xf0,4))[0];assert count<=12
 out.extend(pack(mode));out.extend(data);out.extend(pack(count));out.extend(u.mem_read(result+32,count*16));out.extend(u.mem_read(result,32));out.extend(u.mem_read(result+0xe0,16));out.extend(u.mem_read(result+0x100,8))
p=root/'tests/fixtures/beta-cell-build.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
