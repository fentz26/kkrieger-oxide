#!/usr/bin/env python3
"""Run released Physic mode-1 initialization; stub only instance allocation."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
root=pathlib.Path(__file__).resolve().parent.parent
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x7c0000,(len(image)+4095)&~4095);u.mem_write(0x7c0000,image)
u.mem_map(0x3000000,0x100000)
op,env,game,memory,stack=0x3000000,0x3001000,0x3020000,0x3040000,0x305f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*[x&0xffffffff for x in v])
floats=lambda v:struct.pack('<'+'f'*len(v),*v)
def instance(u,address,size,user):
 esp=u.reg_read(UC_X86_REG_ESP)
 assert struct.unpack('<2I',u.mem_read(esp+4,8))==(op,0x87c)
 u.reg_write(UC_X86_REG_EAX,memory);u.reg_write(UC_X86_REG_EIP,struct.unpack('<I',u.mem_read(esp,4))[0]);u.reg_write(UC_X86_REG_ESP,esp+12)
u.hook_add(UC_HOOK_CODE,instance,begin=0x817b57,end=0x817b57)
rng=random.Random(0x200437);out=bytearray(pack(128))
for case in range(128):
 flags=[1,0x21,0x11,0x31][case%4]
 values=[rng.uniform(-1,1) for _ in range(10)]
 words=[flags]+list(struct.unpack('<10I',floats(values)))+[[0,8,9,10,11,12,13,14,15,255][case%10]]
 matrix=[rng.uniform(-2,2) for _ in range(16)];matrix[3]=matrix[7]=matrix[11]=0.;matrix[15]=1.
 cell=0x12345678 if case%3 else 0;event=0x3060000 if case%5 else 0
 u.mem_write(memory,bytes(0x87c));u.mem_write(memory+0x10,b'\x01')
 u.mem_write(env+0x94f0,floats(matrix));u.mem_write(env+0x133c,pack(event));u.mem_write(env+0x1358,pack(game));u.mem_write(game+0x14960,pack(cell))
 u.mem_write(stack,pack(0x5000000,op,env,*words));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x816d8c,0x816ea6,count=100000)
 out.extend(pack(*words));out.extend(floats(matrix));out.extend(pack(cell,event,game+0x14d44))
 out.extend(u.mem_read(memory+0x14,4));out.extend(u.mem_read(memory+0x33c,1024))
 for index in range(16):
  constraint=list(struct.unpack('<5I',u.mem_read(memory+0x73c+index*20,20)))
  for slot in [2,3]:
   assert (constraint[slot]-memory-0x33c)%128==0
   constraint[slot]=(constraint[slot]-memory-0x33c)//128
  out.extend(pack(*constraint))
p=root/'tests/fixtures/beta-box-projectile.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
