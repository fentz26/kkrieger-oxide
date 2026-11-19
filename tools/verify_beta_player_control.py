#!/usr/bin/env python3
"""Run released pre-physics player translation, with audio output intercepted."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
root=pathlib.Path(__file__).resolve().parent.parent
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x7c0000,(len(image)+4095)&~4095);u.mem_write(0x7c0000,image)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,4096)
pack=lambda *v:struct.pack('<'+'I'*len(v),*[x&0xffffffff for x in v])
floats=lambda v:struct.pack('<'+'f'*len(v),*v)
game,point,frame=0x3000000,0x3020000,0x303f000
sounds=[]
def audio(u,address,size,user):
 esp=u.reg_read(UC_X86_REG_ESP);sounds.append(bytes(u.mem_read(esp+4,16)))
 u.reg_write(UC_X86_REG_EIP,struct.unpack('<I',u.mem_read(esp,4))[0]);u.reg_write(UC_X86_REG_ESP,esp+20)
u.hook_add(UC_HOOK_CODE,audio,begin=0x804a47,end=0x804a47)
rng=random.Random(0x200435);out=bytearray(pack(128))
for case in range(128):
 pos=[rng.uniform(-10,10) for _ in range(3)]+[1.]
 right=[rng.uniform(-1,1) for _ in range(4)];forward=[rng.uniform(-1,1) for _ in range(4)]
 side=rng.uniform(-0.1,0.1);ahead=rng.uniform(-0.1,0.1);speed=rng.uniform(-0.05,0.05)
 gravity=[0.,0.01,-0.01,0.1][case%4];height=[0.2,1.,-0.2][case%3];gain=[0.,0.2,1.][case%3]
 clearance=[0.,0.21,1.01,2.,-1.,float('nan')][case%6]
 off=[0.,0.39,0.4,0.5,float('nan')][case%5];slices=case%8+1
 u.mem_write(game,bytes(0x16000));u.mem_write(point,floats(pos));u.mem_write(frame,pack(0x5000000,point,0,1))
 u.reg_write(UC_X86_REG_ESP,frame);u.reg_write(UC_X86_REG_ECX,game+0x149fc);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81c9c1,0x5000000,count=100000)
 u.mem_write(game+0x15308,floats(right));u.mem_write(game+0x15328,floats(forward))
 u.mem_write(game+0x15354,floats([speed,ahead,side]));u.mem_write(game+0x15394,floats([gravity]))
 u.mem_write(game+0x153a0,floats([gain,height]));u.mem_write(game+0x15510,floats([1.25])+pack(7)+floats([off]))
 u.mem_write(frame-8,floats([float(slices),clearance]));sounds.clear()
 u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_EBX,game);u.reg_write(UC_X86_REG_ESP,frame-0x100)
 u.emu_start(0x81e399,0x81e4a1,count=100000)
 out.extend(floats(pos+right+forward+[side,ahead,speed,gravity,height,gain,clearance,off]));out.extend(pack(slices))
 out.extend(u.mem_read(game+0x149fc,640));out.extend(u.mem_read(game+0x15354,4))
 out.extend(pack(u.mem_read(game+0x15364,1)[0]));out.extend(u.mem_read(game+0x15510,12));out.extend(pack(len(sounds)))
path=root/'tests/fixtures/beta-player-control.bin';path.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
