#!/usr/bin/env python3
"""Run released CollideRaySub2 with all actual hull/vector callees."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000);u.mem_map(0x3100000,0x20000);u.mem_map(0x5000000,0x1000)
cell,end,start,direction,output,stack,game=0x3000000,0x3001000,0x3002000,0x3003000,0x3005000,0x3009000,0x3100000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
fword=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
inside=False
def hook(u,address,size,user):
    global inside
    if address==0x81a2b1:inside=True
u.hook_add(UC_HOOK_CODE,hook)
rng=random.Random(0x200416)
out=bytearray(pack(512));counts=[0,0,0]
for case in range(512):
    mode=[1,9,17,0,2][(case//8)%5];monster=(case//40)%2;interrupt=case%2
    planes=[[1.,0.,0.,1.],[-1.,0.,0.,1.],[0.,1.,0.,1.],[0.,-1.,0.,1.],[0.,0.,1.,1.],[0.,0.,-1.,1.]]
    p0=[-3.,0.,0.,1.];p1=[0.,0.,0.,1.];d=[1.,0.,0.,0.];best=fword(0.0001)
    scenario=case%8
    if scenario==0:p0=[0.,0.,0.,1.];p1=[3.,0.,0.,1.]
    if scenario==2:p1=[3.,0.,0.,1.]
    if scenario==3:p1=[-2.,0.,0.,1.]
    if scenario==4:p0=[-3.,2.,0.,1.];p1=[3.,2.,0.,1.]
    if scenario==5:p1=[0.,3.,0.,1.];d=[0.5**0.5,0.5**0.5,0.,0.]
    if scenario==6:best=fword(-1.)
    if scenario==7:
        p0=[rng.uniform(-5.,5.) for _ in range(3)]+[1.]
        p1=[rng.uniform(-5.,5.) for _ in range(3)]+[1.]
        delta=[b-a for a,b in zip(p0[:3],p1[:3])];length=sum(x*x for x in delta)**0.5
        d=[v/length for v in delta]+[0.]
        best=fword(rng.uniform(-4.,1.))
    count=0 if case==0 else 6
    if case==80:p0[0]=float('nan')
    if case==81:best=0x7fc01234
    if case in [82,83]:p1=[-1.,0.,0.,1.]
    if case==83:best=fword(0.)
    prior=pack(*[rng.getrandbits(32) for _ in range(4)],best)
    geometry=struct.pack('<24f',*[v for plane in planes for v in plane])
    vectors=struct.pack('<12f',*d,*p1,*p0)
    u.mem_write(cell,bytes(0x140));u.mem_write(cell+0x20,geometry);u.mem_write(cell+0xf0,pack(count));u.mem_write(cell+0x100,pack(mode))
    u.mem_write(direction,vectors[:16]);u.mem_write(end,vectors[16:32]);u.mem_write(start,vectors[32:])
    u.mem_write(output,prior);u.mem_write(game+0x15538,pack(monster));u.mem_write(0x14a0980,pack(interrupt))
    u.mem_write(stack,pack(0x5000000,direction,end,start,cell,output,output+16));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game)
    u.reg_write(UC_X86_REG_FPCW,0x143f);inside=False
    u.emu_start(0x81a258,0x5000000,count=10000)
    assert u.reg_read(UC_X86_REG_ESP)==stack+28
    hit=u.reg_read(UC_X86_REG_EAX)&255;assert hit in [0,1]
    counts[2 if inside else hit]+=1
    out.extend(pack(count,mode,monster,interrupt));out.extend(geometry);out.extend(vectors);out.extend(prior)
    out.extend(pack(hit,struct.unpack('<I',u.mem_read(0x14a0980,4))[0],int(inside)));out.extend(u.mem_read(output,20))
path=root/'tests/fixtures/beta-subtractive.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest(),'miss_hit_inside',counts)
