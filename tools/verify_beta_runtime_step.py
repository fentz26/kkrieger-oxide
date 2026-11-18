#!/usr/bin/env python3
"""Execute collider Init then PhysicPart against a real additive hull, no stubs."""
import hashlib,pathlib,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
b=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(b).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);base=0x7c0000
u.mem_map(base,(len(b)+4095)&~4095);u.mem_write(base,b)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,4096)
body,point,cell,array,game,stack=0x3000000,0x3001000,0x3002000,0x3003000,0x3010000,0x303f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
u.mem_write(point,struct.pack('<4f',-0.5,0.,0.,1.));u.mem_write(stack,pack(0x5000000,point,0,3))
u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,body);u.reg_write(UC_X86_REG_FPCW,0x143f)
u.emu_start(0x81c9c1,0x5000000,count=100000)
planes=[[1.,0.,0.,2.],[-1.,0.,0.,2.],[0.,1.,0.,2.],[0.,-1.,0.,2.],[0.,0.,1.,2.],[0.,0.,-1.,2.]]
u.mem_write(cell+32,struct.pack('<24f',*[v for p in planes for v in p]));u.mem_write(cell+0xf0,pack(6))
u.mem_write(array,pack(cell));u.mem_write(game+4,pack(array,1))
u.mem_write(game+0x15390,struct.pack('<4f',0.02,0.01,0.,0.));u.mem_write(game+0x1539c,struct.pack('<f',0.))
u.mem_write(stack,pack(0x5000000,body));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game)
u.emu_start(0x81c341,0x5000000,count=100000)
assert u.reg_read(UC_X86_REG_ESP)==stack+8
assert u.mem_read(body+0x1c,4)==pack(cell)
assert u.mem_read(body+0x14,4)==pack(0)
y=struct.unpack('<I',u.mem_read(body+0x44,4))[0]
assert y==0xbc23d70a
assert u.mem_read(body+0x64,4)==pack(0)
print('Complete native particle/world step: y=0xbc23d70a, history y=positive zero, cell reacquired')
