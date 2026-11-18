#!/usr/bin/env python3
"""Execute released horizontal player-relative direction setup."""
import runpy,pathlib,struct,random,hashlib
from unicorn.x86_const import UC_X86_REG_EAX
s=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_actor_spawn.py')))
globals().update({k:v for k,v in s.items() if not k.startswith('__')})
rng=random.Random(0xd36c);out=bytearray(pack(128))
for case in range(128):
 position=[rng.uniform(-20.,20.) for _ in range(3)]+[1.];player=[rng.uniform(-20.,20.) for _ in range(3)]+[1.]
 if case<8:position=[0.,0.,0.,1.];player=[10.**(-case*3),2.,0.,1.]
 u.mem_write(actor+0x234,struct.pack('<4f',*position));u.mem_write(game+0x14948,struct.pack('<4f',*player))
 u.reg_write(UC_X86_REG_EAX,game);u.reg_write(UC_X86_REG_EBX,actor);u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_ESP,frame-0xb4);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81d36c,0x81d39c,count=1000)
 out.extend(struct.pack('<8f',*position,*player));out.extend(u.mem_read(frame-0x48,16));out.extend(u.mem_read(frame+8,4))
p=root/'tests/fixtures/beta-direction.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
