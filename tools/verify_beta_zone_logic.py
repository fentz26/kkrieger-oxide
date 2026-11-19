#!/usr/bin/env python3
"""Run the released zone-action loop, stopping before weapon-hit dispatch."""
import hashlib
import pathlib
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_FPCW

root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image) + 4095) & ~4095)
u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x40000)
pack = lambda *v: struct.pack('<' + 'I' * len(v), *[x & 0xffffffff for x in v])
game, zone, pointers, stack = 0x3000000, 0x3020000, 0x3021000, 0x303f000
out = bytearray(pack(1664))
for case in range(1664):
    code = ((case % 13) << 4) | ((case // 13) % 16)
    timer = [0, 1, 10, 11, 20, 10000, 0x40000000, 0x7fffffff][(case // 208) % 8]
    output = (case // 13) % 16
    value = (case * 7) % 101 - 20
    masks = [0, 2, 0x100, 0xff02]
    enter, leave, collided = [masks[(case // n) % 4] for n in [1, 4, 16]]
    condition = case % 4
    switches = bytes((case + i) % 5 for i in range(256))
    amounts = [(case + i) % 17 - 3 for i in range(16)]
    maxima = [100 + i * 10 if i < 8 else 1 for i in range(16)]
    use = case % 2
    u.mem_write(game, bytes(0x16000))
    u.mem_write(zone, bytes(512))
    u.mem_write(game + 0x24, pack(pointers, 1))
    u.mem_write(pointers, pack(zone))
    u.mem_write(game + 0x14fe8, switches)
    u.mem_write(game + 0x1496c, pack(3, use))
    u.mem_write(game + 0x1497c, pack(*amounts))
    u.mem_write(game + 0x149bc, pack(*maxima))
    u.mem_write(game + 0x15354, struct.pack('<f', 0.25))
    u.mem_write(zone + 0x108, bytes([code, condition, output, 0]) + pack(value))
    u.mem_write(zone + 0x118, pack(enter, leave, collided, timer))
    u.mem_write(stack, pack(0x5000000, 0))
    u.reg_write(UC_X86_REG_ESP, stack)
    u.reg_write(UC_X86_REG_ECX, game)
    u.reg_write(UC_X86_REG_FPCW, 0x143f)
    u.emu_start(0x81cea2, 0x81d1e0, count=10000)
    out.extend(pack(code, condition, output, value, timer, enter, leave, collided, use))
    out.extend(pack(*switches))
    out.extend(pack(*amounts))
    out.extend(pack(*u.mem_read(game + 0x14fe8, 256)))
    out.extend(u.mem_read(game + 0x1496c, 4))
    out.extend(u.mem_read(game + 0x1497c, 64))
    out.extend(u.mem_read(game + 0x15354, 4))
    out.extend(u.mem_read(zone + 0x118, 16))
path = root / 'tests/fixtures/beta-zone-logic.bin'
path.write_bytes(out)
print(len(out), hashlib.sha256(out).hexdigest())
