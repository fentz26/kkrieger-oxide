#!/usr/bin/env python3
"""Execute released zone traversal and actual OutsideMask calls."""
import hashlib,pathlib,struct,random
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
b=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(b).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);base=0x7c0000
u.mem_map(base,(len(b)+4095)&~4095);u.mem_write(base,b)
u.mem_map(0x3000000,0x10000);u.mem_map(0x5000000,4096)
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
zones,adds,lists,particle,event,start,end,stack=0x3000000,0x3002000,0x3003000,0x3004000,0x3005000,0x3006000,0x3006100,0x300f000
rng=random.Random(0x200420);out=bytearray(pack(256))
for case in range(256):
 kind=[0,1,3,8,35,255][case%6];shot=(case//3)%2;select=[0,1,0xffffffff][(case//6)%3]
 a=[-2.,0.,0.,1.];z=[2.,0.,0.,1.]
 scenario=case%5
 if scenario==1:a=[0.,0.,0.,1.]
 if scenario==2:z=[0.,0.,0.,1.]
 if scenario==3:a=[0.,0.,0.,1.];z=a[:]
 if scenario==4:a=[-2.,2.,0.,1.];z=[2.,2.,0.,1.]
 if case==7:a=[float('nan'),0.,0.,1.]
 masks=[]
 for i in range(3):
  x=i*0.75;p=[[1.,0.,0.,1.-x],[-1.,0.,0.,1.+x],[0.,1.,0.,1.],[0.,-1.,0.,1.],[0.,0.,1.,1.],[0.,0.,-1.,1.]]
  u.mem_write(zones+i*512+32,struct.pack('<24f',*[v for q in p for v in q]));u.mem_write(zones+i*512+0xf0,pack(6))
  m=[rng.getrandbits(32) for _ in range(3)];masks.extend(m);u.mem_write(zones+i*512+0x118,pack(*m))
 ids=[[0,1],[1,2]][(case//7)%2];other=[[2,0],[0,1,2]][(case//11)%2]
 same=(case//5)%2
 for n,seq in enumerate([ids,other]):
  u.mem_write(lists+n*64,pack(*[zones+i*512 for i in seq]));u.mem_write(adds+n*512+0x148,pack(lists+n*64,len(seq)))
 u.mem_write(particle+12,pack(kind));u.mem_write(particle+36,pack(event if shot else 0));u.mem_write(event+48,pack(select))
 vectors=struct.pack('<8f',*a,*z);u.mem_write(start,vectors[:16]);u.mem_write(end,vectors[16:])
 u.mem_write(stack,pack(0x5000000,start,adds,end,adds if same else adds+512,particle))
 u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,0);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81b15a,0x5000000,count=10000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+24
 out.extend(pack(kind,shot,select,same,len(ids),*(ids+[0]*3)[:3],len(other),*(other+[0]*3)[:3]));out.extend(vectors);out.extend(pack(*masks))
 for i in range(3):out.extend(u.mem_read(zones+i*512+0x118,12))
p=root/'tests/fixtures/beta-zones.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
