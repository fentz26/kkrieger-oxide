#!/usr/bin/env python3
"""Execute Particle::Hit, replacing only allocation and event-list insertion."""
import hashlib,pathlib,struct,random
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_EIP,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
b=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(b).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);base=0x7c0000
u.mem_map(base,(len(b)+4095)&~4095);u.mem_write(base,b)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,4096)
particle,event,game,point,plane,stack,emitted,env=0x3000000,0x3001000,0x3010000,0x3002000,0x3002100,0x303f000,0x3003000,0x3004000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
allocated=False;inserted=False

def hook(u,address,size,user):
 global allocated,inserted
 if address not in [0x801000,0x818047]:return
 esp=u.reg_read(UC_X86_REG_ESP)
 if address==0x801000:
  assert read(esp+4)==180 and not allocated;allocated=True;u.reg_write(UC_X86_REG_EAX,emitted);pop=0
 else:
  assert allocated and not inserted and read(esp+4)==emitted and u.reg_read(UC_X86_REG_ECX)==env
  inserted=True;pop=4
 u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+4+pop)
u.hook_add(UC_HOOK_CODE,hook)
rng=random.Random(0x200422);out=bytearray(pack(256));count=0
for case in range(256):
 kind=[7,8,9,15,16][case%5];present=(case//5)%5!=0;reflect=case%2;monster=(case//2)%2
 words=[rng.getrandbits(32) for _ in range(45)];words[2]=[0,1,0xffffffff][case%3];words[12]=[0,1][(case//3)%2]
 time=[0,9,10,11,100,0x7fffffff,0x80000000,0xffffffff][(case//4)%8];words[5]=[0,90,0x7ffffffa,0xfffffffa][(case//9)%4]
 operator=0x12345678 if (case//7)%3 else 0
 if case>=128:
  kind=8+case%8;present=True;words[2]=1;words[12]=0;operator=0x12345678
  words[5]=90;time=100 if case%13==0 else 101
 position=[rng.uniform(-10.,10.) for _ in range(4)];normal=[rng.uniform(-1.,1.) for _ in range(4)]
 if 128<=case<136:normal=[[0.,0.,1.,0.],[1.,0.,0.,0.],[0.,1.,0.,0.],[-0.,-0.,-1.,0.],[-1.,0.,0.,0.],[0.,-1.,0.,0.],[0.,0.,0.,0.],[-0.,-0.,-0.,0.]][case-128]
 vectors=struct.pack('<8f',*position,*normal)
 u.mem_write(particle+12,pack(kind));u.mem_write(particle+36,pack(event if present else 0));u.mem_write(event,pack(*words))
 u.mem_write(game,pack(env));u.mem_write(game+0x15108,bytes(1024));u.mem_write(game+0x15108+kind*4,pack(2 if reflect else 0))
 u.mem_write(game+0x154d0,bytes(64))
 if 8<=kind<16:u.mem_write(game+0x154b0+(kind+monster*8)*4,pack(operator))
 u.mem_write(game+0x15540,pack(time));u.mem_write(point,vectors[:16]);u.mem_write(plane,vectors[16:]);u.mem_write(emitted,bytes([0xa5])*180)
 u.mem_write(stack,pack(0x5000000,game,point,plane,0,monster));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,particle);u.reg_write(UC_X86_REG_FPCW,0x143f)
 allocated=False;inserted=False;u.emu_start(0x81a98d,0x5000000,count=5000)
 assert allocated==inserted and u.reg_read(UC_X86_REG_ESP)==stack+24
 out.extend(pack(kind,int(present),reflect,monster,time,operator,*words));out.extend(vectors)
 out.extend(u.mem_read(event,180));out.extend(pack(int(inserted)));out.extend(u.mem_read(emitted,180) if inserted else bytes(180));count+=int(inserted)
p=root/'tests/fixtures/beta-hit-effect.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest(),'effects',count)
