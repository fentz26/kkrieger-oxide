#!/usr/bin/env python3
"""Run released hit-box local transform construction with its actual callees."""
import hashlib, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EBP, UC_X86_REG_EBX, UC_X86_REG_ESP, UC_X86_REG_FPCW, UC_X86_REG_ECX
root = pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest() == 'ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image = (root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base, start, end = 0x7c0000, 0x81d8cc, 0x81d935
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(base, (len(image)+4095)&~4095); u.mem_write(base, image)
u.mem_map(0x3000000, 0x10000)
actor, words, frame = 0x3000000, 0x3002000, 0x3008000
rng = random.Random(0x200405)
out = bytearray(struct.pack('<I',256))
for case in range(256):
    data = [0.]*26
    data[:3] = [rng.uniform(-3.,3.) for _ in range(3)] if case else [0.]*3
    if 1 <= case <= 8:
        tiny = struct.unpack('<f',struct.pack('<I',[0x80000000,1,0x80000001,0x007fffff,0x807fffff,0x00800000,0x80800000,0x20000000][case-1]))[0]
        data[:3] = [tiny,tiny,tiny]
    data[20:23] = [rng.uniform(-5.,5.) for _ in range(3)]
    data[23:26] = [rng.uniform(-10.,10.) for _ in range(3)]
    packed = struct.pack('<26f',*data)
    u.mem_write(words,packed); u.mem_write(actor+0xbc,packed[:12])
    u.mem_write(frame-8,struct.pack('<I',words))
    u.reg_write(UC_X86_REG_ESP,frame-0x100); u.reg_write(UC_X86_REG_EBP,frame)
    u.reg_write(UC_X86_REG_EBX,actor); u.reg_write(UC_X86_REG_FPCW,0x143f)
    u.emu_start(start,end,count=5000)
    out.extend(packed); out.extend(u.mem_read(frame-0x68,64));out.extend(u.mem_read(frame-0x28,12))
    stack = frame-0x100
    u.mem_write(stack,struct.pack('<4I',0,frame-0x68,frame-0x28,1))
    u.reg_write(UC_X86_REG_ESP,stack); u.reg_write(UC_X86_REG_ECX,actor)
    u.emu_start(0x81c8bb,0x81c945,count=5000)
    esp = u.reg_read(UC_X86_REG_ESP)
    pointer, mode = struct.unpack('<2I',u.mem_read(esp,8))
    assert mode == 1
    out.extend(u.mem_read(pointer,128))
path=root/'tests/fixtures/beta-monster-geometry.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
