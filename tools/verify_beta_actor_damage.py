#!/usr/bin/env python3
"""Execute actual monster Hit, activation lookup and kill-switch increment."""
import runpy,pathlib,struct,random,hashlib
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_actor_spawn.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
from unicorn.x86_const import UC_X86_REG_EAX
rng=random.Random(0x200433);out=bytearray(pack(128));stack=0x305f000
for case in range(128):
 life=[0,-1,1,20,100,0x7fffffff][case%6];armor=[0,3,10,-5][(case//6)%4]
 hits=[0,1,5,30,-5,-2147483648,2147483647][(case//3)%7]
 kill=[0,1,255,-1][(case//7)%4];active=(case//5)%2;present=(case//11)%2
 switches=bytearray(rng.randrange(256) for _ in range(256))
 if kill>0 and case%2:switches[kill]=255
 switches=bytes(switches)
 u.mem_write(actor,bytes(0x7e0));u.mem_write(actor+0x1e4,pack(life&0xffffffff));u.mem_write(actor+0x54,pack(armor&0xffffffff));u.mem_write(actor+0x6c,pack(kill&0xffffffff));u.mem_write(actor+0x1f0,pack(active))
 u.mem_write(actor+0x234,struct.pack('<4f',0.,0.,0.,1.));u.mem_write(game+4,pack(array,present));u.mem_write(game+0x14fe8,switches)
 u.mem_write(stack,pack(0x5000000,hits&0xffffffff,game));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,actor);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81c951,0x5000000,count=10000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+12
 out.extend(pack(life&0xffffffff,armor&0xffffffff,hits&0xffffffff,kill&0xffffffff,active,present));out.extend(switches)
 out.extend(u.mem_read(actor+0x1e4,4));out.extend(u.mem_read(actor+0x1f0,4))
 for i in range(5):out.extend(u.mem_read(actor+0x210+i*128,4))
 out.extend(u.mem_read(game+0x14fe8,256))
p=root/'tests/fixtures/beta-actor-damage.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
