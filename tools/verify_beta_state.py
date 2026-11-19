#!/usr/bin/env python3
"""Execute released State with only its child-execution callback replaced."""
import hashlib, pathlib, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_EIP
root=pathlib.Path(__file__).resolve().parent.parent
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86, UC_MODE_32);u.mem_map(0x7c0000,(len(image)+4095)&~4095);u.mem_write(0x7c0000,image)
u.mem_map(0x3000000,0x50000)
game,env,op,stack,stop=0x3000000,0x3020000,0x3022000,0x3040000,0x304f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*[x&0xffffffff for x in v])
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
children=0
def hook(u,address,size,user):
 global children
 if address!=0x818e4a:return
 children+=1;esp=u.reg_read(UC_X86_REG_ESP)
 assert u.reg_read(UC_X86_REG_ECX)==op and read(esp+4)==env
 u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+8)
u.hook_add(UC_HOOK_CODE,hook);u.mem_write(env+0x1358,pack(game))
keys=[0,32,10,0x10028,8,27,0x10000,0x10001,0x10002,0x10003,0xffffffff,13]
out=bytearray(pack(480));case=0
for condition in range(-1,9):
 for key in keys:
  for variant in range(4):
   before=bytes((case+17*i)&255 for i in range(256))
   parameters=[0 if variant&2 else 31,case*127-512,condition,17,before[17]+(variant&1)]
   u.mem_write(game+0x14fe8,before);u.mem_write(game+0x1553c,pack(key))
   u.mem_write(stack,pack(stop,op,env,*parameters));u.reg_write(UC_X86_REG_ESP,stack)
   children=0;u.emu_start(0x81937b,stop,count=1000)
   assert children==1 and u.reg_read(UC_X86_REG_ESP)==stack+32
   out.extend(pack(case,*parameters,key,read(game+0x1553c)))
   out.extend(u.mem_read(game+0x14fe8,256));case+=1
assert case==480
p=root/'tests/fixtures/beta-state.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
