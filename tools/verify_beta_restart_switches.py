#!/usr/bin/env python3
"""Run released Restart switch/reset writes; replace only player placement."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EIP
root=pathlib.Path(__file__).resolve().parent.parent
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x7c0000,(len(image)+4095)&~4095);u.mem_write(0x7c0000,image)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,4096)
game,stack=0x3000000,0x303f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
placed=0
def place(u,address,size,user):
 global placed
 placed+=1;esp=u.reg_read(UC_X86_REG_ESP)
 assert u.reg_read(UC_X86_REG_ECX)==game
 assert u.mem_read(esp+4,24)==struct.pack('<6f',1.,2.,3.,0.,0.,0.)
 u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+28)
u.hook_add(UC_HOOK_CODE,place,begin=0x81cb71,end=0x81cb71)
rng=random.Random(0x200441);out=bytearray(pack(32))
for case in range(32):
 flags=bytes(rng.randrange(256) for _ in range(256));u.mem_write(game,bytes(0x16000));u.mem_write(game+0x14fe8,flags)
 u.mem_write(game+0x153c4,struct.pack('<4f',1.,2.,3.,0.));u.mem_write(game+0x14948,struct.pack('<4f',-5.,0.,0.,1.));u.mem_write(0x14a0984,pack(0))
 u.mem_write(stack,pack(0x5000000));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game)
 placed=0;u.emu_start(0x81ccee,0x5000000,count=10000)
 assert placed==1 and u.reg_read(UC_X86_REG_ESP)==stack+4
 out.extend(flags);out.extend(u.mem_read(game+0x14fe8,256));out.extend(pack(u.mem_read(game+0x15365,1)[0],read(0x14a0984)))
p=root/'tests/fixtures/beta-restart-switches.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
