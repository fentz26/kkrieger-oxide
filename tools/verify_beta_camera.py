#!/usr/bin/env python3
"""Capture released OnTick camera smoothing with its actual x87 instructions.

Only the straight-line camera tail is executed (0x81e527..0x81e564), with
PC=24/RC=down. No physics, host calls, view-matrix generation or rendering is
substituted or claimed. Cases include finite extremes and repeated updates.
"""
import hashlib, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EBX, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_FPCW
root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image) + 4095) & ~4095)
u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x20000)
game, stack = 0x3000000, 0x301f000
pack = lambda *v: struct.pack('<' + 'I' * len(v), *(v & 0xffffffff for v in v))
word = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]
rng = random.Random(0x200444)
edges = [0, 0x80000000, 1, 0x80000001, 0x007fffff, 0x807fffff,
         0x00800000, 0x80800000, 0x3f800000, 0xbf800000, 0x7f7fffff, 0xff7fffff]
out = bytearray(pack(512))
for case in range(512):
    position = [word(rng.uniform(-100.,100.)) for _ in range(3)] + [word(float(case % 2))]
    previous = word(rng.uniform(-100.,100.))
    gain = word(rng.uniform(-.1,1.1))
    offset = word(rng.uniform(-2.,2.))
    if case >= 144 and case % 4 < 2:
        gain = word([.25, .24999618530273438][case % 4])
    if case < 144:
        position[1] = edges[case % 12]
        previous = edges[(case // 12) % 12]
        gain = word([0., .1, .25, .5, 1., 1.5, -1.][case % 7])
        offset = edges[(case * 5) % 12]
    u.mem_write(game + 0x14948, pack(*position))
    u.mem_write(game + 0x1537c, pack(previous))
    u.mem_write(game + 0x153d4, pack(gain))
    u.mem_write(game + 0x15348, pack(offset))
    steps = 1 if case < 144 else 8
    out.extend(pack(*position, previous, gain, offset, steps))
    for step in range(steps):
        u.reg_write(UC_X86_REG_EBX, game)
        u.reg_write(UC_X86_REG_ESP, stack)
        u.reg_write(UC_X86_REG_FPCW, 0x143f)
        u.emu_start(0x81e527, 0x81e564, count=100)
        assert u.reg_read(UC_X86_REG_EIP) == 0x81e564
        out.extend(u.mem_read(game + 0x1537c, 20))
p = root / 'tests/fixtures/beta-camera.bin'
p.write_bytes(out)
print(len(out), hashlib.sha256(out).hexdigest())
