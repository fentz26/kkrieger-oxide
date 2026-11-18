#!/usr/bin/env python3
"""Released AI life ratio and event selection writeback."""
import runpy,pathlib,struct,random,hashlib
s=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_actor_spawn.py')))
globals().update({k:v for k,v in s.items() if not k.startswith('__')})
rng=random.Random(0xd7b8);out=bytearray(pack(128))
for case in range(128):
 life=rng.randrange(-2147483648,2147483648);maximum=rng.randrange(-2147483648,2147483648)
 if case<8:life=[0,1,-1,2147483647][case%4];maximum=[0,1][case//4]
 u.mem_write(actor+0x1e4,struct.pack('<i',life));u.mem_write(actor+0x50,struct.pack('<i',maximum));u.reg_write(UC_X86_REG_EBX,actor);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81d7b8,0x81d7d1,count=100)
 out.extend(struct.pack('<2i',life,maximum));out.extend(u.mem_read(actor+0xb4,4));out.extend(u.mem_read(actor+0x128,4))
p=root/'tests/fixtures/beta-health-modulation.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
