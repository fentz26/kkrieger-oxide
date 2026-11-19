#!/usr/bin/env python3
"""Execute released Particle::Init and native Sub3, without callee stubs."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x7c0000,(len(image)+4095)&~4095);u.mem_write(0x7c0000,image)
u.mem_map(0x3000000,0x10000);u.mem_map(0x5000000,4096)
particle,position,speed,stack=0x3000000,0x3001000,0x3001100,0x300f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
floats=lambda v:struct.pack('<4f',*v)
rng=random.Random(0x200436);out=bytearray(pack(256))
for case in range(256):
 p=[rng.uniform(-100,100) for _ in range(4)];v=[rng.uniform(-1,1) for _ in range(4)]
 if case<4:p=[0.,-0.,1.,-1.];v=[[0.,0.,0.,0.],[-0.,-0.,-0.,-0.],[1.,-1.,1.,-1.],[-1.,1.,-1.,1.]][case]
 if case==4:p=[float('inf'),float('-inf'),float('nan'),1.];v=p[:]
 point=floats(p);velocity=floats(v);cell=0x12345678 if case%2 else 0
 u.mem_write(particle,bytes([0xa5])*128);u.mem_write(position,point);u.mem_write(speed,velocity)
 u.mem_write(stack,pack(0x5000000,position,speed,cell));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,particle);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81a939,0x5000000,count=10000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+16
 out.extend(point+velocity+pack(cell));out.extend(u.mem_read(particle,128))
p=root/'tests/fixtures/beta-particle-init.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
