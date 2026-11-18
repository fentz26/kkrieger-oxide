#!/usr/bin/env python3
"""Released AI cooldown arithmetic, including the unspilled saturation compare."""
import runpy,pathlib,struct,random,hashlib
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_actor_spawn.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
rng=random.Random(0x1e81ec);out=bytearray(pack(256))
for case in range(256):
 periods=[rng.uniform(-20.,20.) for _ in range(2)]
 timers=[rng.uniform(-1.,2.) for _ in range(2)]
 if case<32:
  periods=[[-1.,0.,1e-30,1e30][case%4], [0.01,0.1,1.,10.][case%4]]
  timers=[[0.,0.5,1.,-0.5][(case//4)%4]]*2
 u.mem_write(actor+0x5c,struct.pack('<f',periods[0]));u.mem_write(actor+0x64,struct.pack('<f',periods[1]));u.mem_write(actor+0x1e8,struct.pack('<2f',*timers))
 u.reg_write(UC_X86_REG_EBX,actor);u.reg_write(UC_X86_REG_ESP,frame-0xb4);u.reg_write(UC_X86_REG_FPCW,0x143f)
 # The gate leaves FLD 1 on the x87 stack immediately before this block.
 u.emu_start(0x81d44c,0x81d452,count=100)
 u.emu_start(0x81d458,0x81d4a4,count=100)
 out.extend(struct.pack('<4f',*periods,*timers));out.extend(u.mem_read(actor+0x1e8,8))
 # Pop the persistent 1 to avoid overflowing the stack across cases.
 u.emu_start(0x81d5db,0x81d5dd,count=10)
p=root/'tests/fixtures/beta-cooldowns.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
