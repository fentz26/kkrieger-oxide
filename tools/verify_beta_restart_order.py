#!/usr/bin/env python3
"""Run released OnTick -> Restart around a parameter update; stub placement/children.

This checks the stored-pose handoff, not native collision placement or the full
menu graph. Initial game storage is explicitly zeroed by this harness.
"""
import hashlib
import pathlib
import random
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import (
    UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_FPCW,
)

root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image) + 4095) & ~4095)
u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x40000)
u.mem_map(0x5000000, 4096)
game, env, op, parameters, stack = 0x3000000, 0x3020000, 0x3022000, 0x3023000, 0x303f000
pack = lambda *v: struct.pack('<' + 'I' * len(v), *[x & 0xffffffff for x in v])
read = lambda p: struct.unpack('<I', u.mem_read(p, 4))[0]
placed = []
children = 0


def hook(u, address, size, user):
    global children
    if address not in (0x81cb71, 0x818e4a):
        return
    esp = u.reg_read(UC_X86_REG_ESP)
    if address == 0x81cb71:
        assert u.reg_read(UC_X86_REG_ECX) == game
        placed.append(bytes(u.mem_read(esp + 4, 24)))
        pop = 24
    else:
        assert u.reg_read(UC_X86_REG_ECX) == op and read(esp + 4) == env
        children += 1
        pop = 4
    u.reg_write(UC_X86_REG_EIP, read(esp))
    u.reg_write(UC_X86_REG_ESP, esp + 4 + pop)


u.hook_add(UC_HOOK_CODE, hook)
rng = random.Random(0x200442)
out = bytearray(pack(32))
for case in range(32):
    u.mem_write(game, bytes(0x16000))
    before = [0., 0., 0., 0.] if case == 0 else [rng.uniform(-4, 4), 2., rng.uniform(-4, 4), 0.]
    words = [0] * 57
    after = [rng.uniform(-4, 4), 2., rng.uniform(-4, 4)]
    words[29:32] = struct.unpack('<3I', struct.pack('<3f', *after))
    u.mem_write(game + 0x153c4, struct.pack('<4f', *before))
    u.mem_write(env + 0x1358, pack(game))
    u.mem_write(op + 0x94, pack(parameters))
    u.mem_write(parameters, pack(*words))
    counter = [0, 0x7ffffffe, 0xfffffffe, 0x80000000][case % 4]
    u.mem_write(game + 0x15540, pack(counter))
    out.extend(struct.pack('<4f', *before))
    out.extend(pack(*words))
    out.extend(pack(counter))
    for phase in range(2):
        if phase:
            u.mem_write(stack, pack(0x5000000, op, env))
            u.reg_write(UC_X86_REG_ESP, stack)
            u.reg_write(UC_X86_REG_FPCW, 0x143f)
            children = 0
            u.emu_start(0x81a35e, 0x5000000, count=10000)
            assert children == 1 and u.reg_read(UC_X86_REG_ESP) == stack + 12
        # Both cell guards must pass; no cell is dereferenced in this branch.
        u.mem_write(game + 0x14960, pack(1))
        u.mem_write(game + 0x14a18, pack(1))
        u.mem_write(game + 0x14fe9, bytes([2]))
        u.mem_write(0x14a0984, pack(0))
        u.mem_write(stack, pack(0x5000000, env, 3))
        u.reg_write(UC_X86_REG_ESP, stack)
        u.reg_write(UC_X86_REG_ECX, game)
        placed = []
        u.emu_start(0x81dd4c, 0x5000000, count=10000)
        assert len(placed) == 1 and u.reg_read(UC_X86_REG_ESP) == stack + 12
        assert placed[0] == bytes(u.mem_read(game + 0x153c4, 16)) + bytes(8)
        assert u.mem_read(game + 0x14fe9, 1) == bytes(1)
        out.extend(placed[0])
        out.extend(pack(read(game + 0x15540), u.mem_read(game + 0x15365, 1)[0]))
p = root / 'tests/fixtures/beta-restart-order.bin'
p.write_bytes(out)
print(len(out), hashlib.sha256(out).hexdigest())
