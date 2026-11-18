#!/usr/bin/env python3
"""Execute released second-contact math, including opposing and crease paths."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_ESP,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000)
frame,particle=0x3008000,0x3000000
branch=0
branches={0x81b7de:1,0x81b819:2,0x81b859:3}
def hook(u,address,size,user):
    global branch
    if address in branches:branch=branches[address]
u.hook_add(UC_HOOK_CODE,hook)
rng=random.Random(0x200411)
out=bytearray(struct.pack('<I',256));counts=[0,0,0]
for case in range(256):
    target=[rng.uniform(-10.,10.) for _ in range(4)]
    direction=[rng.uniform(-1.,1.) for _ in range(3)]
    inverse=rng.uniform(0.03,4.)
    n1=[1.,0.,0.]
    choice=case%5
    if choice==0:n2=[-1.,0.,0.]
    elif choice==1:n2=[1.,0.,0.]
    elif choice==2:n2=[0.,1.,0.]
    elif choice==3:n2=[-0.5,0.75**0.5,0.]
    else:
        n1=[rng.uniform(-1.,1.) for _ in range(3)];n2=[rng.uniform(-1.,1.) for _ in range(3)]
        n1=[v/sum(x*x for x in n1)**0.5 for v in n1]
        n2=[v/sum(x*x for x in n2)**0.5 for v in n2]
    if case<6:
        x=struct.unpack('<f',struct.pack('<I',[0xbf666665,0xbf666666,0xbf666667,0x38d1b716,0x38d1b717,0x38d1b718][case]))[0]
        n1=[1.,0.,0.];n2=[x,(1.-x*x)**0.5,0.]
    p1=n1+[rng.uniform(-3.,3.)];p2=n2+[rng.uniform(-3.,3.)]
    distance=rng.uniform(-2.,0.)
    old=[rng.uniform(-10.,10.) for _ in range(4)]
    if case in [6,7,8]:
        target=[1.,0.,0.,1.];old=target[:];distance=0.
        p1=[1.,0.,0.,-1.]
        p2={6:[1.,0.,0.,-1.],7:[-1.,0.,0.,1.],8:[0.,1.,0.,0.]}[case]
    args=struct.pack('<21f',*target,*direction,inverse,*p1,*p2,distance,*old)
    u.mem_write(particle+0x40,bytes(16));u.mem_write(particle+0x50,args[68:84]);u.mem_write(particle+0x60,bytes(16))
    u.mem_write(frame-0x1c,args[:16]);u.mem_write(frame-0x84,args[16:28]);u.mem_write(frame-0x20,args[28:32])
    u.mem_write(frame-0x94,args[32:48]);u.mem_write(frame-0xa4,args[48:64]);u.mem_write(frame-0x74,args[64:68])
    u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_EBX,particle);u.reg_write(UC_X86_REG_ESP,frame-0x200)
    u.reg_write(UC_X86_REG_FPCW,0x143f);branch=0
    u.emu_start(0x81b799,0x81ba21,count=3000)
    assert branch in [1,2,3];counts[branch-1]+=1
    out.extend(args);out.extend(struct.pack('<I',int(branch==1)))
    out.extend(u.mem_read(frame-0x30,16));out.extend(u.mem_read(frame-0x1c,16));out.extend(u.mem_read(frame-0x84,12))
    out.extend(u.mem_read(particle+0x40,16));out.extend(u.mem_read(particle+0x60,16))
path=root/'tests/fixtures/beta-second-slide.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest(), 'branches',counts)
