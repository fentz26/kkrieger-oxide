#!/usr/bin/env python3
"""Execute the complete released collider initializer, with no callee stubs."""
import hashlib, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000);u.mem_map(0x5000000,0x1000)
body,point,stack=0x3000000,0x3008000,0x3009000
rng=random.Random(0x200407)
out=bytearray(struct.pack('<I',256))
for case in range(256):
    position=[rng.uniform(-100.,100.) for _ in range(3)]+[1.]
    if case<4:position=[[0.,0.,0.,1.],[-0.,-0.,-0.,0.],[1.,-1.,1.,1.],[-1.,1.,-1.,1.]][case]
    packed=struct.pack('<4f',*position)
    cell=0 if case%2==0 else 0x12345678
    kind=case%4
    u.mem_write(body,bytes([0xa5])*0x600);u.mem_write(point,packed)
    u.mem_write(stack,struct.pack('<4I',0x5000000,point,cell,kind))
    u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,body);u.reg_write(UC_X86_REG_FPCW,0x143f)
    u.emu_start(0x81c9c1,0x5000000,count=100000)
    assert u.reg_read(UC_X86_REG_ESP)==stack+16
    out.extend(packed);out.extend(struct.pack('<3I',cell,kind,body))
    out.extend(u.mem_read(body,0x348))
path=root/'tests/fixtures/beta-collider.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
