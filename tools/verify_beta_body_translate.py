#!/usr/bin/env python3
"""Execute released collider position translation, retaining every other field."""
import runpy,pathlib,struct,random,hashlib
s=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in s.items() if not k.startswith('__')})
rng=random.Random(0x819d55);out=bytearray(pack(128))
for case in range(128):
 initial=bytearray(640)
 for i in range(5):
  struct.pack_into('<I',initial,i*128, int(i==4))
  struct.pack_into('<28f',initial,i*128+16,*[rng.uniform(-100.,100.) for _ in range(28)])
 delta=struct.pack('<4f',*[rng.uniform(-1.,1.) for _ in range(4)])
 u.mem_write(body,bytes(initial));u.mem_write(0x300e000,delta);u.mem_write(stack,pack(0x5000000,0x300e000));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,body);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x819d55,0x5000000,count=10000)
 out.extend(initial);out.extend(delta);out.extend(u.mem_read(body,640))
p=root/'tests/fixtures/beta-body-translate.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
