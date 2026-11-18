#!/usr/bin/env python3
"""Execute complete PhysicDCell; stub only its mode-1 collision probe."""
import runpy,pathlib,struct,random,hashlib
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_EIP
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
# Use a fresh emulator so hooks also cover code executed by the setup verifier.
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(b)+4095)&~4095);u.mem_write(base,b)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,4096)
dynamic,particle,matrix=0x3004000,0x3005000,0x3006000
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
calls=0;probe=bytes(16)
def hook(u,address,size,user):
 global calls,probe
 if address!=0x81b5bd:return
 esp=u.reg_read(UC_X86_REG_ESP);assert read(esp+4)==particle and read(esp+8)==1
 calls+=1;assert calls==1;probe=bytes(u.mem_read(particle+0x40,16))
 u.mem_write(particle+0x40,changes[:16]);u.mem_write(particle+0x50,changes[16:32]);u.mem_write(particle+0x60,changes[32:48])
 u.reg_write(UC_X86_REG_EAX,contact);u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+12)
u.hook_add(UC_HOOK_CODE,hook)
rng=random.Random(0x200428);out=bytearray(pack(128))
geometry=struct.pack('<24f',1,0,0,1,-1,0,0,1,0,1,0,1,0,-1,0,1,0,0,1,1,0,0,-1,1)
identity=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.]
for case in range(128):
 confirmed=identity[:];current=identity[:];current[12:15]=[rng.uniform(-0.3,0.3) for _ in range(3)]
 if case%8==0:current=confirmed[:]
 kind=[3,8,15,16][case%4];owned=(case//4)%3==0;monster=(case//7)%2;contact=case%3
 position=[rng.uniform(-0.5,0.5) for _ in range(3)]+[1.]
 if case%11==0:position=[3.,3.,3.,1.]
 changes=struct.pack('<12f',*[rng.uniform(-2.,2.) for _ in range(12)])
 u.mem_write(dynamic,bytes(0x2a4));u.mem_write(dynamic+0xf0,pack(6));u.mem_write(dynamic+0x13c,geometry)
 u.mem_write(matrix,struct.pack('<16f',*confirmed));u.mem_write(stack,pack(0x5000000,matrix));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,dynamic);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81a880,0x5000000,count=10000)
 u.mem_write(dynamic+0x100,pack(0x21));u.mem_write(dynamic+0x1a0,pack(0x12345678 if monster else 0));u.mem_write(dynamic+0x1a4,struct.pack('<16f',*confirmed));u.mem_write(dynamic+0x1e4,struct.pack('<16f',*current))
 raw=[0]*32;raw[0]=1;raw[3]=kind;raw[7]=cell;raw[8]=dynamic if owned else 0;raw[11]=struct.unpack('<I',struct.pack('<f',1.))[0]
 raw[16:20]=struct.unpack('<4I',struct.pack('<4f',*position));raw[20:24]=raw[16:20];raw[24:28]=raw[16:20];raw[28:]=raw[16:20]
 u.mem_write(particle,pack(*raw));u.mem_write(game+0x44,pack(particle));u.mem_write(game+0x14844,pack(1))
 u.mem_write(stack,pack(0x5000000,dynamic));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game)
 calls=0;probe=bytes(16);u.emu_start(0x81c4c4,0x5000000,count=50000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+8
 out.extend(pack(kind,int(owned),monster,contact));out.extend(struct.pack('<4f',*position));out.extend(struct.pack('<32f',*confirmed,*current));out.extend(changes)
 out.extend(pack(calls));out.extend(probe);out.extend(u.mem_read(particle+0x40,48))
 out.extend(pack(read(dynamic+0x100),read(dynamic+0x138)));out.extend(u.mem_read(dynamic+0x20,96));out.extend(u.mem_read(dynamic+0xe0,16));out.extend(u.mem_read(dynamic+0x1a4,128))
p=root/'tests/fixtures/beta-dynamic-reconcile.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
