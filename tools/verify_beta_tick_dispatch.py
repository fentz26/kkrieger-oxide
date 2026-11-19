#!/usr/bin/env python3
"""Run the released OnTick prologue, observing restart/quit and stopping at gameplay."""
import hashlib,pathlib,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EIP
root=pathlib.Path(__file__).resolve().parent.parent
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x7c0000,(len(image)+4095)&~4095);u.mem_write(0x7c0000,image)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,4096)
game,env,system,stack=0x3000000,0x3020000,0x3021000,0x303f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*[x&0xffffffff for x in v])
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
action=0
def hook(u,address,size,user):
 global action
 if address==0x81ddac:action=1;u.emu_stop();return
 if address not in [0x81ccee,0x80102e]:return
 action=2 if address==0x81ccee else 3
 assert u.reg_read(UC_X86_REG_ECX)==(game if action==2 else system)
 esp=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+4)
u.hook_add(UC_HOOK_CODE,hook)
u.mem_write(0x83f868,pack(system));out=bytearray(pack(1024))
for present in range(4):
 for mode in range(256):
  before=[0,0x7ffffffe,0xfffffffe,0x80000000][mode%4];slices=[0,1,5,0xffffffff][mode%4]
  u.mem_write(game+0x14960,pack(0x12345678 if present&1 else 0));u.mem_write(game+0x14a18,pack(0x12345678 if present&2 else 0));u.mem_write(game+0x14fe9,bytes([mode]));u.mem_write(game+0x15540,pack(before))
  u.mem_write(stack,pack(0x5000000,env,slices));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game)
  action=0;u.emu_start(0x81dd4c,0x5000000,count=1000)
  out.extend(pack(present,mode,before,slices,action,read(game+0x15540)))
p=root/'tests/fixtures/beta-tick-dispatch.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
