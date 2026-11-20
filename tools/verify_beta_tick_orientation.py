#!/usr/bin/env python3
"""Capture the released OnTick player-matrix and hit-cell orientation spans.

Two straight-line spans run with their real Euler callee (0x805a9d) under
PC=24/RC=down: 0x81de61..0x81dea1 builds PlayerMatrix from the fly switch,
look and direction; 0x81e4a1..0x81e4f3 builds the hit-cell matrix, copies the
player position and raises it by 0.85. Input, physics, the following cell
update and camera code are not executed or claimed.
"""
import hashlib, math, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EBX, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_FPCW
root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
body = lambda a, z: hashlib.sha256(image[a - 0x7c0000:z - 0x7c0000]).hexdigest()
assert body(0x81de61, 0x81dea1) == '40ef502ede01c4e3733c811d72da429cc771fe8921485045253a184f993e9431'
assert body(0x81e4a1, 0x81e4f3) == '430626ec59bb4601df759aba6fbb142fe571153c5ca277b0edbb701c8ee9dc35'
assert body(0x805a9d, 0x805b55) == 'cce14228aaca32063c43087c12bf82ba92e824e0c19b0acdbdaeb405111ccd20'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image) + 4095) & ~4095)
u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x20000)
game, stack = 0x3000000, 0x301f000
pack = lambda *v: struct.pack('<' + 'I' * len(v), *(v & 0xffffffff for v in v))
word = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]
rng = random.Random(0x81de61)
edges = [0., -0., 1e-30, -1e-30, .5, -.5, math.pi / 2, -math.pi / 2, math.pi,
         -math.pi, 2 * math.pi, 1.5707964, 3.1415927, 100., -1000., 0.19495817]
heights = [0, 0x80000000, 1, 0xbf59999a, 0x3f59999a, 0x7f7fffff, 0xff7fffff, 0x00800000]
cases = 512
out = bytearray(pack(cases))
for case in range(cases):
    look, direction = rng.uniform(-1.6, 1.6), rng.uniform(-7., 7.)
    if case < 256:
        look, direction = edges[case % 16], edges[(case // 16) % 16]
    fly = [0, 1, 0, 0x80000000][(case // 2) % 4]
    position = [word(rng.uniform(-200., 200.)) for _ in range(3)] + [rng.getrandbits(32)]
    if case % 8 == 7:
        position[1] = heights[(case // 8) % 8]
    u.mem_write(game + 0x14948, pack(*position, word(look), word(direction)))
    u.mem_write(game + 0x15360, pack(fly))
    out.extend(pack(word(look), word(direction), fly, *position))
    for start, stop, result in [(0x81de61, 0x81dea1, 0x15308), (0x81e4a1, 0x81e4f3, 0x14fa8)]:
        u.mem_write(game + result, pack(*(0xc0de0000 + n for n in range(16))))
        u.reg_write(UC_X86_REG_EBX, game)
        u.reg_write(UC_X86_REG_ESP, stack)
        u.reg_write(UC_X86_REG_FPCW, 0x143f)
        u.emu_start(start, stop, count=1000)
        assert u.reg_read(UC_X86_REG_EIP) == stop
        assert u.reg_read(UC_X86_REG_ESP) == stack
        out.extend(u.mem_read(game + result, 64))
p = root / 'tests/fixtures/beta-tick-orientation.bin'
p.write_bytes(out)
print(len(out), hashlib.sha256(out).hexdigest())
