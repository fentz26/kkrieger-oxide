#!/usr/bin/env python3
"""Capture the released GetCamera routine (0x819e7b) with its actual callees.

The complete routine runs under the game's PC=24/RC=down control word,
including the released Euler initializer (0x805a9d) and the zoom-position
AddScale3 (0x80604f). Nothing is substituted. Transcendentals are evaluated by
the emulator, as in the other fsin/fcos fixtures; projection, viewport
operators and rendering are not executed or claimed.
"""
import hashlib, math, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_FPCW
root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
body = lambda a, z: hashlib.sha256(image[a - 0x7c0000:z - 0x7c0000]).hexdigest()
assert body(0x819e7b, 0x819f28) == 'a42473eff241884baa37182599addcac74482e354dbb602c544a9c12ad808dff'
assert body(0x805a9d, 0x805b55) == 'cce14228aaca32063c43087c12bf82ba92e824e0c19b0acdbdaeb405111ccd20'
assert body(0x80604f, 0x80607a) == 'd13d33b4aa11ddb05b27a568876e6e0ea2b3aa7ef5d6b092f63295bef511629f'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image) + 4095) & ~4095)
u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x30000)
game, env, stack, done = 0x3000000, 0x3020000, 0x302f000, 0x302ff00
pack = lambda *v: struct.pack('<' + 'I' * len(v), *(v & 0xffffffff for v in v))
word = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]
rng = random.Random(0x819e7b)
edges = [0., -0., 1e-30, -1e-30, .5, -.5, math.pi / 2, -math.pi / 2, math.pi,
         -math.pi, 2 * math.pi, 1.5707964, 3.1415927, 100., -1000., 0.19495817]
cases = 512
out = bytearray(pack(cases))
for case in range(cases):
    look, direction = rng.uniform(-1.6, 1.6), rng.uniform(-7., 7.)
    if case < 256:
        look, direction = edges[case % 16], edges[(case // 16) % 16]
    zoom = [0, 1, 0, 0x80000000][case % 4] if case >= 256 else (case // 128) % 2
    player = [word(rng.uniform(-200., 200.)) for _ in range(3)] + [rng.getrandbits(32)]
    camera = [word(rng.uniform(-200., 200.)) for _ in range(3)] + [rng.getrandbits(32)]
    if case % 64 == 63:
        player[:3] = [0x80000000, 1, 0x7f7fffff]
    u.mem_write(game + 0x14948, pack(*player, word(look), word(direction), 0x5eed0001, zoom))
    u.mem_write(game + 0x15380, pack(*camera))
    u.mem_write(env + 0x11c, pack(*(0xc0de0000 + n for n in range(23))))
    u.mem_write(stack, pack(done, env))
    u.reg_write(UC_X86_REG_ECX, game)
    u.reg_write(UC_X86_REG_ESP, stack)
    u.reg_write(UC_X86_REG_FPCW, 0x143f)
    u.emu_start(0x819e7b, done, count=1000)
    assert u.reg_read(UC_X86_REG_EIP) == done
    assert u.reg_read(UC_X86_REG_ESP) == stack + 8
    assert u.reg_read(UC_X86_REG_FPCW) == 0x143f
    out.extend(pack(*player, word(look), word(direction), zoom, *camera))
    out.extend(u.mem_read(env + 0x11c, 92))
p = root / 'tests/fixtures/beta-get-camera.bin'
p.write_bytes(out)
print(len(out), hashlib.sha256(out).hexdigest())
