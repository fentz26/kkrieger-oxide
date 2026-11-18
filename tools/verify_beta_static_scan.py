#!/usr/bin/env python3
"""Execute complete CollideRaySub with static lists and zero dynamic cells."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x40000);u.mem_map(0x3100000,0x20000);u.mem_map(0x5000000,0x1000)
cells,adds,arrays,visited,particle,direction,end,start,output,stack,game=0x3000000,0x3010000,0x3014000,0x3015000,0x3020000,0x3021000,0x3022000,0x3023000,0x3025000,0x3039000,0x3100000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
fword=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
rng=random.Random(0x200417)
shapes=[(0.,1.),(0.5,1.),(-0.25,1.),(0.25,0.25),(1.5,0.5),(-1.,0.5)]
patterns=[[],[0],[1],[2,3],[0,0,1],[3,1,0],[1,3],[2,0,2]]
out=bytearray(pack(256));counts=[0,0]
for case in range(256):
    kind=[None,0,1,2,3,4,8][(case//8)%7];monster=(case//7)%2;interrupt=(case//11)%2
    modes=[1,9,1,[0,2,17,9,1][(case//9)%5],1,9]
    only=[0,case%2,1,(case//3)%2,0,0]
    geometry=[]
    for i,(center,extent) in enumerate(shapes):
        planes=[[1.,0.,0.,extent-center],[-1.,0.,0.,extent+center],
            [0.,1.,0.,1.],[0.,-1.,0.,1.],[0.,0.,1.,1.],[0.,0.,-1.,1.]]
        data=struct.pack('<24f',*[v for p in planes for v in p]);geometry.append(data)
        u.mem_write(cells+i*0x200,bytes(0x140));u.mem_write(cells+i*0x200+0x20,data)
        u.mem_write(cells+i*0x200+0xf0,pack(6));u.mem_write(cells+i*0x200+0x100,pack(modes[i],only[i]))
    lists=[[4,0,1],[2,0],[3,4,5],[1,5]]
    for i,ids in enumerate(lists):
        u.mem_write(adds+i*0x200,bytes(0x140));u.mem_write(adds+i*0x200+0x138,pack(arrays+i*0x100,len(ids)))
        u.mem_write(arrays+i*0x100,pack(*[cells+j*0x200 for j in ids]))
    walk=patterns[case%8]
    if walk:u.mem_write(visited,pack(*[adds+i*0x200 for i in walk]))
    p0=[-3.,0.,0.,1.];p1=[3.,0.,0.,1.];d=[1.,0.,0.,0.]
    if case%13==0:p0=[0.,0.,0.,1.]
    if case%13==1:p0=[-3.,3.,0.,1.];p1=[3.,3.,0.,1.]
    if case%13==2:p1=[-2.,0.,0.,1.]
    if case%17==0:p0=[-3.,-3.,0.,1.];p1=[3.,3.,0.,1.];d=[0.5**0.5,0.5**0.5,0.,0.]
    vectors=struct.pack('<12f',*d,*p1,*p0)
    prior=pack(*[rng.getrandbits(32) for _ in range(4)],fword(rng.uniform(-20.,20.)))
    p_arg=0 if kind is None else particle;first=p_arg if case%2 else 0
    u.mem_write(particle+0xc,pack(kind or 0));u.mem_write(game+0x15538,pack(monster));u.mem_write(game+0x1484c,pack(0))
    u.mem_write(direction,vectors[:16]);u.mem_write(end,vectors[16:32]);u.mem_write(start,vectors[32:])
    u.mem_write(output,prior);u.mem_write(0x14a0980,pack(interrupt))
    u.mem_write(stack,pack(0x5000000,direction,end,start,visited,len(walk),output,output+16,p_arg,first))
    u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game);u.reg_write(UC_X86_REG_FPCW,0x143f)
    u.emu_start(0x81b213,0x5000000,count=30000)
    assert u.reg_read(UC_X86_REG_ESP)==stack+40
    hit=u.reg_read(UC_X86_REG_EAX)&255;assert hit in [0,1];counts[hit]+=1
    out.extend(pack(0xffffffff if kind is None else kind,monster,interrupt,int(first!=0)))
    out.extend(vectors);out.extend(prior)
    for mode,flag,data in zip(modes,only,geometry):out.extend(pack(mode,flag));out.extend(data)
    out.extend(pack(len(walk),*(walk+[0]*3)[:3]))
    out.extend(pack(hit,struct.unpack('<I',u.mem_read(0x14a0980,4))[0]));out.extend(u.mem_read(output,20))
path=root/'tests/fixtures/beta-static-scan.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest(),'miss_hit',counts)
