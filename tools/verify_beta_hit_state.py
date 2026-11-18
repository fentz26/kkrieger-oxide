#!/usr/bin/env python3
"""Execute original Particle::Hit with null effect slots, compare event words."""
import hashlib,pathlib,struct,random
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
b=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(b).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);base=0x7c0000
u.mem_map(base,(len(b)+4095)&~4095);u.mem_write(base,b)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,4096)
particle,event,game,point,plane,stack=0x3000000,0x3001000,0x3010000,0x3002000,0x3002100,0x303f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
rng=random.Random(0x200421);out=bytearray(pack(256))
for case in range(256):
 kind=[0,1,3,8,9,15,16,35,255][case%9];present=(case//3)%2;reflect=case%2;monster=(case//2)%2
 words=[rng.getrandbits(32) for _ in range(45)];time=rng.getrandbits(32)
 u.mem_write(particle+12,pack(kind));u.mem_write(particle+36,pack(event if present else 0));u.mem_write(event,pack(*words))
 u.mem_write(game+0x15108,bytes(1024));u.mem_write(game+0x15108+kind*4,pack(2 if reflect else 0));u.mem_write(game+0x15540,pack(time))
 u.mem_write(stack,pack(0x5000000,game,point,plane,0,monster));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,particle);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81a98d,0x5000000,count=3000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+24
 out.extend(pack(present,reflect,monster,time,*words));out.extend(u.mem_read(event,180))
p=root/'tests/fixtures/beta-hit-state.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
