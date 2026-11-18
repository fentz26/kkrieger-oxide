#!/usr/bin/env python3
"""Released terrain-height correction after the downward cast."""
import runpy,pathlib,struct,random,hashlib
s=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_actor_spawn.py')))
globals().update({k:v for k,v in s.items() if not k.startswith('__')})
rng=random.Random(0xd6d4);out=bytearray(pack(256))
for case in range(256):
 values=[rng.uniform(-5.,5.) for _ in range(5)]
 gravity,height,gain,clearance,vertical=values
 for offset,value in [(0x15394,gravity),(0x153a4,height),(0x153a0,gain)]:u.mem_write(game+offset,struct.pack('<f',value))
 u.mem_write(frame+8,struct.pack('<f',clearance));u.mem_write(frame-0x14,struct.pack('<f',vertical));u.reg_write(UC_X86_REG_ESI,game);u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81d6d4,0x81d70b,count=100)
 out.extend(struct.pack('<5f',*values));out.extend(u.mem_read(frame-0x14,4))
p=root/'tests/fixtures/beta-height.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
