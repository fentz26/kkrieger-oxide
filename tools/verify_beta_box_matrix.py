#!/usr/bin/env python3
"""Execute released PartBox::MakeMatrix without replacing callees."""
import hashlib, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_FPCW
root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image)+4095)&~4095); u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x20000); u.mem_map(0x5000000, 4096)
body, matrix, stack = 0x3000000, 0x3001000, 0x301f000
pack = lambda *v: struct.pack('<'+'I'*len(v), *v)
floats = lambda v: struct.pack('<'+'f'*len(v), *v)
rng = random.Random(0x200439)
out = bytearray(pack(256))
for case in range(256):
    positions = [[rng.uniform(-10,10) for _ in range(4)] for _ in range(8)]
    if case % 16 == 0: positions = [[0.,0.,0.,1.] for _ in range(8)]
    if case % 16 == 1: positions = [[3.,-2.,1.,1.] for _ in range(8)]
    for index, position in enumerate(positions): u.mem_write(body+index*128+64, floats(position))
    u.mem_write(matrix, bytes([0xa5])*64); u.mem_write(stack, pack(0x5000000,matrix))
    u.reg_write(UC_X86_REG_ESP,stack); u.reg_write(UC_X86_REG_ECX,body); u.reg_write(UC_X86_REG_FPCW,0x143f)
    u.emu_start(0x81abbd,0x5000000,count=10000)
    assert u.reg_read(UC_X86_REG_ESP)==stack+8
    out.extend(floats([v for position in positions for v in position])); out.extend(u.mem_read(matrix,64))
p=root/'tests/fixtures/beta-box-matrix.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
