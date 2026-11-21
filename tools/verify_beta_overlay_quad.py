#!/usr/bin/env python3
"""Capture released FXQuad and its actual projection; stub device submissions only.

Runs FXQuad 0x81439e..0x81451b, environment initialization and projection
0x8022a7. Material/geometry device boundaries are recorded stubs, not GPU output.
"""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ECX,UC_X86_REG_ESP,UC_X86_REG_EIP,UC_X86_REG_EAX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x7c0000,(len(image)+4095)&~4095);u.mem_write(0x7c0000,image)
u.mem_map(0x3000000,0x20000);u.mem_map(0x5000000,4096)
manager,system,uv0,uv1,vertices,environment,projection,stack=0x3000000,0x3004000,0x3005000,0x3005100,0x3006000,0x3007000,0x3008000,0x301f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v))
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
trace=[];captured=b''
def hook(u,address,size,user):
 global captured
 sp=u.reg_read(UC_X86_REG_ESP)
 args=lambda n:[read(sp+4+i*4) for i in range(n)]
 trace.append(address)
 if address==0x804b25:
  a=args(4);assert a[1]==manager+0xe8+material*0xf0 and a[3]==0
  captured=bytes(u.mem_read(a[2],0x180));pop=16
 elif address==0x80479f:
  a=args(6);assert a==[77,4,0,a[3],0,3]
  u.mem_write(a[3],pack(vertices));pop=24
 elif address==0x801856:
  assert args(3)==[77,0xffffffff,0xffffffff];pop=12
 else:
  assert args(1)==[manager+0xd4];pop=4
 assert u.reg_read(UC_X86_REG_ECX)==system
 u.reg_write(UC_X86_REG_EIP,read(sp));u.reg_write(UC_X86_REG_ESP,sp+4+pop);u.reg_write(UC_X86_REG_EAX,0)
for address in [0x804b25,0x80479f,0x801856,0x8016d1]:u.hook_add(UC_HOOK_CODE,hook,begin=address,end=address)
u.mem_write(0x83f868,pack(system));u.mem_write(manager+0xd4,pack(77))
rng=random.Random(0x200445);out=bytearray(pack(64))
sizes=[1,3,7,16,256,480,640,1024,1920,4096,0,-1,2147483647]
for case in range(64):
 width,height=sizes[case%len(sizes)],sizes[(case*3)%len(sizes)]
 material=case%6
 coords=[rng.uniform(-1.,2.) for _ in range(8)]
 u.mem_write(uv0,struct.pack('<8f',*coords));u.mem_write(uv1,struct.pack('<4f',*coords[4:]))
 u.mem_write(system+0x204,pack(width,height));u.mem_write(vertices,bytes([0xa5])*128)
 u.mem_write(stack,pack(0x5000000,material,uv0 if case%4 else 0,uv1 if case%5 else 0))
 u.reg_write(UC_X86_REG_ECX,manager);u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_FPCW,0x143f)
 trace=[];captured=b'';u.emu_start(0x81439e,0x5000000,count=10000)
 assert u.reg_read(UC_X86_REG_EIP)==0x5000000 and u.reg_read(UC_X86_REG_ESP)==stack+16
 assert trace==[0x804b25,0x80479f,0x801856,0x8016d1]
 out.extend(pack(width,height,case%4!=0,case%5!=0));out.extend(struct.pack('<8f',*coords))
 out.extend(pack(struct.unpack_from('<i',captured,0x174)[0]));out.extend(u.mem_read(vertices,128))
 u.mem_write(environment,captured);u.mem_write(stack,pack(0x5000000,projection))
 u.reg_write(UC_X86_REG_ECX,environment);u.reg_write(UC_X86_REG_ESP,stack)
 u.emu_start(0x8022a7,0x5000000,count=1000)
 assert u.reg_read(UC_X86_REG_EIP)==0x5000000 and u.reg_read(UC_X86_REG_ESP)==stack+8
 out.extend(u.mem_read(projection,64))
p=root/'tests/fixtures/beta-overlay-quad.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
