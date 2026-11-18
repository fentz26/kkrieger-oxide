#!/usr/bin/env python3
"""Execute released steering clamp and read its vector scale argument."""
import runpy,pathlib,struct,random,hashlib
s=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_actor_spawn.py')))
globals().update({k:v for k,v in s.items() if not k.startswith('__')})
rng=random.Random(0xd5dd);out=bytearray(pack(256))
for case in range(256):
 values=[rng.uniform(-10.,10.) for _ in range(5)]
 distance,speed,threshold,near,far=values
 u.mem_write(actor+0x40,struct.pack('<f',speed));u.mem_write(actor+0x48,struct.pack('<f',threshold));u.mem_write(actor+0x70,struct.pack('<2f',near,far));u.mem_write(frame+8,struct.pack('<f',distance))
 u.reg_write(UC_X86_REG_EBX,actor);u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_ESP,frame-0xb4);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.mem_write(frame-0x48,struct.pack('<4f',0.6,0.,0.8,0.));u.mem_write(actor+0x234,struct.pack('<4f',1000.,2.,-1000.,1.));u.mem_write(frame-0x18,struct.pack('<4f',1000.,2.,-1000.,1.))
 u.emu_start(0x81d5dd,0x81d644,count=1000)
 out.extend(struct.pack('<5f',*values));out.extend(u.mem_read(frame-0x18,16))
p=root/'tests/fixtures/beta-displacement.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
