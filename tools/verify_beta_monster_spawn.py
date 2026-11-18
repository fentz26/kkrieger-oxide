#!/usr/bin/env python3
"""Run the released spawn prefix, including actual event init and spline lookup."""
import hashlib, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EBP, UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_ESP, UC_X86_REG_FPCW
root = pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest() == 'ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image = (root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base = 0x7c0000
u = Uc(UC_ARCH_X86,UC_MODE_32)
u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x40000)
actor,op,data,array,env,frame = 0x3000000,0x3001000,0x3002000,0x3003000,0x3010000,0x3030000
rng = random.Random(0x200406)
out = bytearray(struct.pack('<I',256))
fword = lambda f: struct.unpack('<I',struct.pack('<f',f))[0]
for case in range(256):
    words = [rng.getrandbits(32) for _ in range(26)]
    words[:7] = [fword(rng.uniform(-100.,100.)) for _ in range(7)]
    parent = [fword(rng.uniform(-100.,100.)) for _ in range(16)]
    if case < 8:
        words[3:6] = [0x80000000,0x3f800000,1]
        parent[12:15] = [0,0xbf800000,0xbf800000]
    spline = 0 if case % 2 == 0 else 0x31415926
    packed = struct.pack('<26I',*words)
    parent_bytes = struct.pack('<16I',*parent)
    u.mem_write(actor,bytes([0xa5])*0x800)
    u.mem_write(data,packed)
    u.mem_write(op+0x94,struct.pack('<I',data))
    u.mem_write(op+0x54,struct.pack('<I',int(spline != 0)))
    u.mem_write(op+0x7c,struct.pack('<I',array));u.mem_write(array,struct.pack('<I',spline))
    u.mem_write(env+0x94f0,parent_bytes)
    u.mem_write(frame+8,struct.pack('<2I',op,env))
    u.reg_write(UC_X86_REG_ESP,frame-0x100);u.reg_write(UC_X86_REG_EBP,frame)
    u.reg_write(UC_X86_REG_EBX,actor);u.reg_write(UC_X86_REG_ESI,op)
    u.reg_write(UC_X86_REG_FPCW,0x143f)
    u.emu_start(0x81d81e,0x81d968,count=5000)
    out.extend(packed);out.extend(parent_bytes);out.extend(struct.pack('<2I',spline,actor))
    out.extend(u.mem_read(actor,0x40));out.extend(u.mem_read(actor+0x40,56))
    out.extend(u.mem_read(actor+0x78,180));out.extend(u.mem_read(actor+0x12c,180))
    out.extend(u.mem_read(actor+0x1e0,8))
    # FindCell returns with RET 0x10: the trailing word is retained for
    # Collider::Init(position, cell, type), not an argument of FindCell.
    query = u.mem_read(u.reg_read(UC_X86_REG_ESP),20)
    assert image[0x81b152-base:0x81b155-base] == bytes.fromhex('c21000')
    assert struct.unpack_from('<I',query,16)[0] == 3
    out.extend(query)
path=root/'tests/fixtures/beta-monster-spawn.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
