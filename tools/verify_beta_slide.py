#!/usr/bin/env python3
"""Run released travel normalization and first-plane projection math slices."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_ESP,UC_X86_REG_EIP,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000)
frame,particle=0x3008000,0x3000000
normalizing=False;early=False
def hook(u,address,size,user):
    global early
    if normalizing and address==0x81baa3:
        early=True;u.reg_write(UC_X86_REG_EIP,0x81b64b)
u.hook_add(UC_HOOK_CODE,hook)
rng=random.Random(0x200410)
out=bytearray(struct.pack('<I',256))
for case in range(256):
    start=[rng.uniform(-20.,20.) for _ in range(4)]
    end=[rng.uniform(-20.,20.) for _ in range(4)]
    if case<8:
        start=[0.,0.,0.,1.];end=[0.,0.,[0.,1e-12,1e-9,1e-8,1.000001e-8,2e-8,1.,-1.][case],1.]
    normal=[rng.uniform(-1.,1.) for _ in range(3)]
    length=sum(x*x for x in normal)**0.5
    plane=[x/length for x in normal]+[rng.uniform(-2.,2.)]
    distance=rng.uniform(-3.,0.)
    if case in [8,9]:
        start=[0.,0.,0.,1.];end=[1.,0.,0.,1.]
        plane=[1.,0.,0.,-1.];distance=0. if case==8 else -1e-12
    inputs=struct.pack('<13f',*start,*end,*plane,distance)
    u.mem_write(frame-0x30,inputs[:16]);u.mem_write(frame-0x1c,inputs[16:32])
    u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_EBX,particle)
    u.reg_write(UC_X86_REG_ESP,frame-0x200);u.reg_write(UC_X86_REG_FPCW,0x143f)
    normalizing=True;early=False
    u.emu_start(0x81b5fb,0x81b64b,count=1000)
    normalizing=False
    out.extend(inputs);out.extend(struct.pack('<I',int(not early)))
    if early:
        u.emu_start(0x81baa3,0x81baa5,count=1)
        out.extend(bytes(76));continue
    initial=bytes(u.mem_read(frame-0x84,12))+bytes(u.mem_read(frame-0x20,4))
    u.mem_write(frame-4,inputs[48:]);u.mem_write(frame-0x94,inputs[32:48])
    u.emu_start(0x81b693,0x81b6b3,count=1000)
    impact=bytes(u.mem_read(frame-0x30,16))
    u.emu_start(0x81b6d6,0x81b75c,count=1000)
    out.extend(initial);out.extend(impact);out.extend(u.mem_read(frame-0x30,16))
    out.extend(u.mem_read(frame-0x1c,16));out.extend(u.mem_read(frame-0x84,12))
path=root/'tests/fixtures/beta-slide.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
