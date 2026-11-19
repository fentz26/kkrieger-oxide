#!/usr/bin/env python3
"""Run complete original Physic passes for one box body and no dynamic cells."""
import runpy,pathlib,struct,random,hashlib
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
rng=random.Random(0x200423);out=bytearray(pack(32))
for case in range(32):
 matrix,scale,speed=0x3001100,0x3001200,0x3001300
 m=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,-0.5,0.,0.,1.]
 u.mem_write(matrix,struct.pack('<16f',*m));u.mem_write(scale,struct.pack('<3f',1.,1.,1.));u.mem_write(speed,struct.pack('<4f',0.05,0.01,0.,0.))
 u.mem_write(body,bytes(0x540));u.mem_write(stack,pack(0x5000000,matrix,scale,speed,0));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,body);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81aaa4,0x5000000,count=100000)
 for i in range(8):u.mem_write(body+i*128+12,pack(8));u.mem_write(body+i*128+44,struct.pack('<f',1.))
 forces=struct.pack('<32f',*[rng.uniform(-0.01,0.01) for _ in range(32)])
 for i in range(8):u.mem_write(body+i*128+0x30,forces[i*16:i*16+16])
 settings=struct.pack('<4f',0.02,0.01,0.,0.)
 u.mem_write(game+0x15390,settings);u.mem_write(game+0x1539c,struct.pack('<f',0.))
 u.mem_write(game+0x44,pack(body));u.mem_write(game+0x10044,pack(body+0x400));u.mem_write(game+0x14844,pack(1,1,0))
 passes=1+case%4
 for _ in range(passes):
  u.mem_write(stack,pack(0x5000000,0x3f800000));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game);u.reg_write(UC_X86_REG_FPCW,0x143f)
  u.emu_start(0x81c728,0x5000000,count=200000)
  assert u.reg_read(UC_X86_REG_ESP)==stack+8
 out.extend(pack(passes));out.extend(settings);out.extend(forces);out.extend(u.mem_read(body,1024))
p=root/'tests/fixtures/beta-box-pass.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
