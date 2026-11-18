#!/usr/bin/env python3
"""Execute full Physic with 1..3 body/constraint registrations, no dynamic cells."""
import runpy,pathlib,struct,random,hashlib
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
rng=random.Random(0x200424);out=bytearray(pack(24))
for case in range(24):
 n=1+case%3;addresses=[0x3004000+i*0x800 for i in range(n)];initial=[]
 for i,address in enumerate(addresses):
  position=struct.pack('<4f',-0.5+i*0.25,0.,0.,1.);forces=struct.pack('<20f',*[rng.uniform(-0.01,0.01) for _ in range(20)])
  u.mem_write(point,position);u.mem_write(stack,pack(0x5000000,point,0,3));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,address)
  u.emu_start(0x81c9c1,0x5000000,count=100000)
  for j in range(5):u.mem_write(address+j*128+0x30,forces[j*16:j*16+16])
  initial.append(position+forces)
 u.mem_write(game+0x44,pack(*addresses));u.mem_write(game+0x10044,pack(*[p+0x280 for p in addresses]));u.mem_write(game+0x14844,pack(n,n,0))
 passes=1+case%4
 for _ in range(passes):
  u.mem_write(stack,pack(0x5000000,0x3f800000));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game);u.reg_write(UC_X86_REG_FPCW,0x143f)
  u.emu_start(0x81c728,0x5000000,count=300000)
  assert u.reg_read(UC_X86_REG_ESP)==stack+8
 out.extend(pack(n,passes))
 for address,data in zip(addresses,initial):out.extend(pack(address));out.extend(data);out.extend(u.mem_read(address,640))
p=root/'tests/fixtures/beta-multi-body.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
