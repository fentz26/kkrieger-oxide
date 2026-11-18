#!/usr/bin/env python3
"""Run complete original Physic passes for one body and no dynamic cells."""
import runpy,pathlib,struct,random,hashlib
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
rng=random.Random(0x200423);out=bytearray(pack(32))
for case in range(32):
 u.mem_write(stack,pack(0x5000000,point,0,3));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,body)
 u.emu_start(0x81c9c1,0x5000000,count=100000)
 forces=struct.pack('<20f',*[rng.uniform(-0.01,0.01) for _ in range(20)])
 for i in range(5):u.mem_write(body+i*128+0x30,forces[i*16:i*16+16])
 settings=struct.pack('<4f',0.02,0.01,0.,0.)
 u.mem_write(game+0x15390,settings);u.mem_write(game+0x1539c,struct.pack('<f',0.))
 u.mem_write(game+0x44,pack(body));u.mem_write(game+0x10044,pack(body+0x280));u.mem_write(game+0x14844,pack(1,1,0))
 passes=1+case%4
 for _ in range(passes):
  u.mem_write(stack,pack(0x5000000,0x3f800000));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game);u.reg_write(UC_X86_REG_FPCW,0x143f)
  u.emu_start(0x81c728,0x5000000,count=200000)
  assert u.reg_read(UC_X86_REG_ESP)==stack+8
 out.extend(pack(passes));out.extend(settings);out.extend(forces);out.extend(u.mem_read(body,640))
p=root/'tests/fixtures/beta-body-pass.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
