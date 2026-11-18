#!/usr/bin/env python3
"""Execute released attack event modulation writeback."""
import runpy,pathlib,struct,random,hashlib
s=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_actor_spawn.py')))
globals().update({k:v for k,v in s.items() if not k.startswith('__')})
rng=random.Random(0xd762);out=bytearray(pack(128))
for case in range(128):
 timers=[rng.uniform(-2.,2.) for _ in range(2)] if case>7 else [[0.,0.5],[1.,-0.5],[0.49999997,0.50000006],[2.,-2.]][case%4]
 u.mem_write(actor+0x1e8,struct.pack('<2f',*timers));u.reg_write(UC_X86_REG_EBX,actor);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81d44c,0x81d452,count=10);u.emu_start(0x81d762,0x81d7b8,count=100)
 out.extend(struct.pack('<2f',*timers));out.extend(u.mem_read(actor+0xac,8))
p=root/'tests/fixtures/beta-attack-modulation.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
