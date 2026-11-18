#!/usr/bin/env python3
"""Execute released attack selection/event construction; intercept world services."""
import runpy,pathlib,struct,hashlib
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
s=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_actor_spawn.py')))
globals().update({k:v for k,v in s.items() if not k.startswith('__')})
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(b)+4095)&~4095);u.mem_write(base,b);u.mem_map(0x3000000,0x60000)
event=0x3040000
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
def ret(n=0,value=0):
 esp=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EAX,value);u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+4+n)
def hook(u,address,size,user):
 global damage,casts,emitted
 esp=u.reg_read(UC_X86_REG_ESP)
 if address==0x819cf4:
  damage=read(esp+4);ret(4)
 elif address==0x81c852:
  assert struct.unpack('<4f',u.mem_read(read(esp+4),16))==(2.,4.,4.,1.)
  assert struct.unpack('<4f',u.mem_read(read(esp+8),16))==(5.,7.,7.,1.)
  casts+=1;ret(16,blocked)
 elif address==0x801000:
  assert read(esp+4)==180;u.mem_write(event,bytes(180));ret(0,event)
 elif address==0x818047:
  assert read(esp+4)==event;emitted=1;ret(4)
u.hook_add(UC_HOOK_CODE,hook)
out=bytearray(pack(384))
for case in range(384):
 distance=[1.,1.5,2.,float('nan')][case%4]
 timers=[0.,1.,2.,float('nan')]
 melee=timers[(case//4)%4];ranged=timers[(case//16)%4]
 enabled=(case//64)%3;blocked=(case//192)%2
 op=0x12345678 if enabled==2 else 0;hits=7 if enabled else 0
 u.mem_write(actor,bytes(0x7e0));u.mem_write(actor+0x58,pack(hits));u.mem_write(actor+0x60,pack(hits));u.mem_write(actor+0x1e0,pack(op));u.mem_write(actor+0x1e8,struct.pack('<2f',melee,ranged))
 # Distinct source event rotation/translation, copied verbatim to the new event.
 source=pack(*range(0x3f800000,0x3f800008));u.mem_write(actor+0xbc,source)
 u.mem_write(game+0x14948,struct.pack('<4f',2.,3.,4.,1.));u.mem_write(actor+0x30,struct.pack('<4f',5.,6.,7.,1.))
 u.mem_write(frame+8,struct.pack('<f',distance));u.mem_write(frame+12,pack(0x3050000));u.mem_write(frame-4,pack(game))
 u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_ESP,frame-0xb4);u.reg_write(UC_X86_REG_EBX,actor);u.reg_write(UC_X86_REG_EDI,game);u.reg_write(UC_X86_REG_ESI,0);u.reg_write(UC_X86_REG_FPCW,0x143f)
 damage=casts=emitted=0
 u.emu_start(0x81d44c,0x81d452,count=10);u.emu_start(0x81d4a4,0x81d5dd,count=10000)
 out.extend(struct.pack('<3f',distance,melee,ranged));out.extend(pack(hits,op,blocked,damage,casts,emitted));out.extend(u.mem_read(actor+0x1e8,8))
 if emitted:out.extend(u.mem_read(event,180))
p=root/'tests/fixtures/beta-attack.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
