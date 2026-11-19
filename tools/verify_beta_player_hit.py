#!/usr/bin/env python3
"""Run released Player::Hit; intercept only the audio output call."""
import hashlib
import pathlib
import random
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_EIP
root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image) + 4095) & ~4095)
u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x10000)
u.mem_map(0x5000000, 4096)
pack = lambda *v: struct.pack('<' + 'I' * len(v), *[x & 0xffffffff for x in v])
player, stack = 0x3000000, 0x300f000
sounds = []
def audio(u, address, size, user):
    esp = u.reg_read(UC_X86_REG_ESP)
    sounds.append(bytes(u.mem_read(esp + 4, 16)))
    u.reg_write(UC_X86_REG_EIP, struct.unpack('<I', u.mem_read(esp, 4))[0])
    u.reg_write(UC_X86_REG_ESP, esp + 20)
u.hook_add(UC_HOOK_CODE, audio, begin=0x804a47, end=0x804a47)
rng = random.Random(0x200434)
edges = [-2147483648, -2147483647, -100, -5, -1, 0, 1, 4, 5, 100, 2147483646, 2147483647]
out = bytearray(pack(1984))
for case in range(1984):
    values = [edges[(case // n) % len(edges)] if case < 1728 else rng.randrange(-2147483648, 2147483648) for n in [1, 12, 144]]
    life, armor, hits = values
    u.mem_write(player, bytes(128))
    u.mem_write(player + 0x14, pack(life, armor))
    u.mem_write(stack, pack(0x5000000, hits))
    u.reg_write(UC_X86_REG_ESP, stack)
    u.reg_write(UC_X86_REG_ECX, player)
    sounds.clear()
    u.emu_start(0x819cf4, 0x5000000, count=10000)
    assert u.reg_read(UC_X86_REG_ESP) == stack + 8
    out.extend(pack(life, armor, hits))
    out.extend(u.mem_read(player + 0x14, 8))
    out.extend(pack(len(sounds)))
    for sound in sounds:
        out.extend(sound)
path = root / 'tests/fixtures/beta-player-hit.bin'
path.write_bytes(out)
print(len(out), hashlib.sha256(out).hexdigest())
