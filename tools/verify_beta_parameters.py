#!/usr/bin/env python3
"""Execute released parameter updates; replace only child scene execution."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EIP,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x7c0000,(len(image)+4095)&~4095);u.mem_write(0x7c0000,image)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,4096)
op,env,game,parameters,stack=0x3000000,0x3001000,0x3010000,0x3004000,0x303f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
children=0
def child(u,address,size,user):
 global children
 children+=1;esp=u.reg_read(UC_X86_REG_ESP)
 assert u.reg_read(UC_X86_REG_ECX)==op and read(esp+4)==env
 assert u.mem_read(game+0x153c4,12)==u.mem_read(parameters+29*4,12)
 assert read(game+0x153d0)==0
 u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+8)
u.hook_add(UC_HOOK_CODE,child,begin=0x818e4a,end=0x818e4a)
rng=random.Random(0x200440);out=bytearray(pack(64,3))
fields=[0x15394,0x1539c,0x15398,0x153a0,0x153a4,0x153a8,0x153ac,0x153b0,0x153b4,0x153b8,0x153bc,0x153c0]
for case in range(64):
 u.mem_write(game,bytes(0x16000))
 for call in range(3):
  reset=call!=1;words=[rng.getrandbits(32) for _ in range(57)]
  for i in list(range(17,32))+list(range(40,48))+[56]:words[i]=struct.unpack('<I',struct.pack('<f',rng.uniform(-4,4)))[0]
  if case%8==0:words[22]=words[21]
  u.mem_write(parameters,pack(*words));u.mem_write(op+0x94,pack(parameters));u.mem_write(env+0x1358,pack(game));u.mem_write(game+0x15365,bytes([reset]))
  u.mem_write(stack,pack(0x5000000,op,env));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_FPCW,0x143f)
  children=0;u.emu_start(0x81a35e,0x5000000,count=10000)
  assert children==1 and u.reg_read(UC_X86_REG_ESP)==stack+12
  out.extend(pack(int(reset),*words));out.extend(pack(u.mem_read(game+0x15365,1)[0]))
  out.extend(u.mem_read(game+0x1497c,32));out.extend(u.mem_read(game+0x149bc,32));out.extend(u.mem_read(game+0x1499c,32))
  out.extend(pack(*[read(game+offset) for offset in fields]));out.extend(u.mem_read(game+0x153c4,20));out.extend(u.mem_read(game+0x150e8,96));out.extend(u.mem_read(game+0x15348,4))
p=root/'tests/fixtures/beta-parameters.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
