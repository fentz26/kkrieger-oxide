#!/usr/bin/env python3
"""Verify portal/static-obstacle integration with the full released world cast."""
import hashlib,pathlib,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
b=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(b).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);base=0x7c0000
u.mem_map(base,(len(b)+4095)&~4095);u.mem_write(base,b)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,4096)
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
def make(cell,x,extent):
 p=[[1.,0.,0.,extent-x],[-1.,0.,0.,extent+x],[0.,1.,0.,1.],[0.,-1.,0.,1.],[0.,0.,1.,1.],[0.,0.,-1.,1.]]
 u.mem_write(cell+32,struct.pack('<24f',*[v for q in p for v in q]));u.mem_write(cell+0xf0,pack(6))
a,c,obstacle,game,particle,stack=0x3000000,0x3000200,0x3000400,0x3010000,0x3005000,0x303f000
make(a,0.,1.);make(c,2.,1.);make(obstacle,1.5,0.25);u.mem_write(obstacle+0x100,pack(1))
for cell,neighbor,offset in [(a,c,0),(c,a,16)]:
 u.mem_write(0x3006000+offset,pack(neighbor));u.mem_write(cell+0x128,pack(0x3006000+offset,1))
 u.mem_write(cell+0x138,pack(0x3006100,1))
u.mem_write(0x3006100,pack(obstacle));u.mem_write(particle+12,pack(3))
u.mem_write(0x3007000,struct.pack('<4f',1.,0.,0.,0.));u.mem_write(0x3007100,struct.pack('<4f',2.,0.,0.,1.))
for start,current,expected,interrupt in [(0.,a,[-1.,-0.,-0.,1.25,-0.75],0),(1.5,c,[1.,0.,0.,0.,0.],1)]:
 u.mem_write(0x3007200,struct.pack('<4f',start,0.,0.,1.));u.mem_write(0x3007300,pack(current));u.mem_write(0x14a0980,pack(0))
 u.mem_write(stack,pack(0x5000000,0x3007000,0x3007100,0x3007200,0x3007300,0x3007400,0x3007410,particle,0))
 u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81b45a,0x5000000,count=10000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+36 and u.reg_read(UC_X86_REG_EAX)&255==1
 assert u.mem_read(0x3007300,4)==pack(c)
 assert u.mem_read(0x3007400,20)==struct.pack('<5f',*expected)
 assert u.mem_read(0x14a0980,4)==pack(interrupt)
print('2 complete released world-cast scenarios passed, no callee stubs')
