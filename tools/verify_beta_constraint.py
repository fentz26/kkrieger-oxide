#!/usr/bin/env python3
"""Execute the actual released PhysicCons and its math callees."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000);u.mem_map(0x5000000,0x1000)
a,b,cons,stack=0x3000000,0x3000100,0x3000200,0x3009000
rng=random.Random(0x200408)
out=bytearray(struct.pack('<I',512))
for case in range(512):
    pa=[rng.uniform(-20.,20.) for _ in range(4)]
    pb=[rng.uniform(-20.,20.) for _ in range(4)]
    rest=rng.uniform(0.,10.)
    if case<4:
        pa=[0.,-0.,0.,1.];pb=pa[:];rest=[0.,1.,0.5,100.][case]
    if case%8 and case>=8:
        pa=list(struct.unpack('<4f',u.mem_read(a+0x40,16)))
        pb=list(struct.unpack('<4f',u.mem_read(b+0x40,16)))
    args=struct.pack('<9f',*pa,*pb,rest)
    u.mem_write(a+0x40,args[:16]);u.mem_write(b+0x40,args[16:32])
    alias = case % 17 == 0
    u.mem_write(cons,struct.pack('<4I',0,0,a,a if alias else b)+args[32:])
    u.mem_write(stack,struct.pack('<2I',0x5000000,cons));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_FPCW,0x143f)
    u.emu_start(0x819f7c,0x5000000,count=1000)
    assert u.reg_read(UC_X86_REG_ESP)==stack+8
    out.extend(args);out.extend(struct.pack('<I',int(alias)));out.extend(u.mem_read(a+0x40,16));out.extend(u.mem_read(b+0x40,16))
path=root/'tests/fixtures/beta-constraint.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
