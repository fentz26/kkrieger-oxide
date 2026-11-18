#!/usr/bin/env python3
"""Execute released FindCell with its actual OutsideMask callee."""
import hashlib,pathlib,struct,random
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
b=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(b).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);base=0x7c0000
u.mem_map(base,(len(b)+4095)&~4095);u.mem_write(base,b)
u.mem_map(0x3000000,0x10000);u.mem_map(0x5000000,4096)
manager,array,cells,stack=0x3000000,0x3001000,0x3002000,0x300f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
rng=random.Random(0x200419);out=bytearray(pack(256))
for case in range(256):
 count=case%7
 planes=[]
 for i in range(6):
  x=i*1.5
  p=[[1.,0.,0.,1.-x],[-1.,0.,0.,1.+x],[0.,1.,0.,1.],[0.,-1.,0.,1.],[0.,0.,1.,1.],[0.,0.,-1.,1.]]
  planes.append(p)
  u.mem_write(cells+i*512+32,struct.pack('<24f',*[v for q in p for v in q]));u.mem_write(cells+i*512+0xf0,pack(6))
 order=list(range(6));rng.shuffle(order)
 u.mem_write(array,pack(*[cells+i*512 for i in order]));u.mem_write(manager+4,pack(array,count))
 point=[rng.uniform(-2.,10.),rng.uniform(-2.,2.),0.,1.]
 if case<6:point=[case*1.5+1.,0.,0.,1.]
 if case==7:point=[float('nan'),0.,0.,1.]
 data=struct.pack('<4f',*point)
 u.mem_write(stack,pack(0x5000000)+data);u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,manager);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81b115,0x5000000,count=5000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+20
 ptr=u.reg_read(UC_X86_REG_EAX);answer=0xffffffff if ptr==0 else (ptr-cells)//512
 out.extend(pack(count,*order));out.extend(data);out.extend(pack(answer))
p=root/'tests/fixtures/beta-find-cell.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
