#!/usr/bin/env python3
"""Execute released PartBox::Init with no replaced callees."""
import hashlib, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_FPCW
root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image)+4095)&~4095)
u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x20000)
u.mem_map(0x5000000, 4096)
body, matrix, scale, speed, stack = 0x3000000, 0x3001000, 0x3001100, 0x3001200, 0x301f000
pack = lambda *v: struct.pack('<'+'I'*len(v), *v)
floats = lambda v: struct.pack('<'+'f'*len(v), *v)
rng = random.Random(0x200438)
out = bytearray(pack(128))
for case in range(128):
    m = [rng.uniform(-2, 2) for _ in range(16)]
    m[3] = m[7] = m[11] = 0.; m[15] = 1.
    size = [rng.uniform(-3, 3) for _ in range(3)]
    velocity = [rng.uniform(-1, 1) for _ in range(4)]
    if case % 8 == 0: size = [0., -0., 1.]
    cell = 0x12345678 if case % 3 else 0
    inputs = floats(m+size+velocity)+pack(cell)
    u.mem_write(matrix, floats(m)); u.mem_write(scale, floats(size)); u.mem_write(speed, floats(velocity))
    u.mem_write(body, bytes(0x540))
    u.mem_write(stack, pack(0x5000000, matrix, scale, speed, cell))
    u.reg_write(UC_X86_REG_ESP, stack); u.reg_write(UC_X86_REG_ECX, body); u.reg_write(UC_X86_REG_FPCW, 0x143f)
    u.emu_start(0x81aaa4, 0x5000000, count=100000)
    assert u.reg_read(UC_X86_REG_ESP) == stack+20
    out.extend(inputs); out.extend(u.mem_read(body, 0x400))
    for index in range(16):
        words = list(struct.unpack('<5I', u.mem_read(body+0x400+index*20, 20)))
        for slot in [2, 3]:
            assert body <= words[slot] < body+0x400 and (words[slot]-body)%128 == 0
            words[slot] = (words[slot]-body)//128
        out.extend(pack(*words))
p = root / 'tests/fixtures/beta-box-init.bin'
p.write_bytes(out)
print(len(out), hashlib.sha256(out).hexdigest())
