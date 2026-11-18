#!/usr/bin/env python3
"""Released event position and facing writeback."""
import runpy,pathlib,struct,random,hashlib
s=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_actor_spawn.py')))
globals().update({k:v for k,v in s.items() if not k.startswith('__')})
rng=random.Random(0xd720);out=bytearray(pack(128))
for case in range(128):
 position=[rng.uniform(-20.,20.) for _ in range(3)]+[1.]
 direction=[rng.uniform(-1.,1.),0.,rng.uniform(-1.,1.),0.]
 u.mem_write(actor+0x234,struct.pack('<4f',*position));u.mem_write(frame-0x48,struct.pack('<4f',*direction))
 u.reg_write(UC_X86_REG_EBX,actor);u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_ESP,frame-0xb4);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81d44c,0x81d452,count=10);u.emu_start(0x81d720,0x81d762,count=100)
 out.extend(struct.pack('<8f',*position,*direction));out.extend(u.mem_read(actor+0xbc,32))
 # Pop the persistent constant to keep the x87 stack balanced.
 u.emu_start(0x81d5db,0x81d5dd,count=10)
p=root/'tests/fixtures/beta-facing.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
