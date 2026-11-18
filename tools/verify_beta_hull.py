#!/usr/bin/env python3
"""Execute released OutsideMask/IntersectOut/IntersectIn and actual math callees."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000);u.mem_map(0x5000000,0x1000)
cell,point,direction,result,stack=0x3000000,0x3001000,0x3002000,0x3005000,0x3009000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
fword=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
rng=random.Random(0x200414)
out=bytearray(pack(512))
for case in range(512):
    count=case%13
    p=[rng.uniform(-20.,20.) for _ in range(4)];d=[rng.uniform(-2.,2.) for _ in range(4)]
    radius=rng.uniform(-1.,1.);planes=[]
    for _ in range(12):
        n=[rng.uniform(-1.,1.) for _ in range(3)];length=sum(x*x for x in n)**0.5
        planes.append([x/length for x in n]+[rng.uniform(-10.,10.)])
    if 1<=case<=12:
        planes=[[0.,0.,0.,0.] for _ in range(12)];planes[0]=[1.,0.,0.,-1.]
        p=[0.,0.,0.,1.];d=[1.,0.,0.,1.];radius=0.
        if case==1:d=[0.,0.,0.,1.];radius=0.5
        if case==2:planes[1]=[-1.,0.,0.,-1.];d=[0.,0.,0.,1.]
        if case==3:p[0]=0.75;radius=0.5
        if case in [4,5]:
            planes[1]=[0.,1.,0.,-1.];d=[-1.,-1.,0.,1.] if case==4 else [1.,1.,0.,1.]
        if case in [6,7]:planes[0][3]=0. if case==6 else -0.
        if case==8:planes[0]=[0.,0.,0.,-1.];d=[0.,0.,0.,1.]
        if case==9:planes[0]=[-1.,0.,0.,-1.];d=[0.,0.,0.,1.]
        if case in [10,11,12]:
            bits=fword(1e-16)+(case-11)
            d=[struct.unpack('<f',pack(bits))[0],0.,0.,1.]
    if case in [14,15,16]:
        planes=[[0.,0.,0.,0.] for _ in range(12)]
        planes[0]=[1.,-1.,0.,0.];p=[float('inf'),float('inf'),0.,1.]
        d=[1.,0.,0.,1.];radius=-1.
        if case==15:planes[0]=[-1.,1.,0.,0.];p=[-float('inf'),-float('inf'),0.,1.]
        if case==16:p=[float('nan'),0.,0.,1.]
    inputs=pack(count)+struct.pack('<9f',*p,*d,radius)+struct.pack('<48f',*[v for plane in planes for v in plane])
    u.mem_write(cell,bytes(0x140));u.mem_write(cell+0x20,inputs[40:]);u.mem_write(cell+0xf0,pack(count))
    u.mem_write(point,inputs[4:20]);u.mem_write(direction,inputs[20:36])
    u.mem_write(stack,pack(0x5000000,point,fword(radius)))
    u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,cell);u.reg_write(UC_X86_REG_FPCW,0x143f)
    u.emu_start(0x819fea,0x5000000,count=3000)
    assert u.reg_read(UC_X86_REG_ESP)==stack+12
    out.extend(inputs);out.extend(pack(u.reg_read(UC_X86_REG_EAX)))
    for entry in [0x81a047,0x81a0e3]:
        u.mem_write(result,bytes([0xa5])*20)
        u.mem_write(stack,pack(0x5000000,point,direction,result,result+16))
        u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,cell)
        u.emu_start(entry,0x5000000,count=3000)
        assert u.reg_read(UC_X86_REG_ESP)==stack+20
        hit=u.reg_read(UC_X86_REG_EAX)&255;assert hit in [0,1]
        out.extend(pack(hit));out.extend(u.mem_read(result,20))
path=root/'tests/fixtures/beta-hull.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
