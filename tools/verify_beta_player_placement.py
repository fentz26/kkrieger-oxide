#!/usr/bin/env python3
"""Execute released SetPlayer, FindCell and full particle/hit-cell construction.

Only host mouse-input retrieval is replaced. Cells are built by original code;
fixture cases cover an empty world, overlapping cells and outside positions.
This is bounded placement evidence, not full menu scheduling or a playthrough.
"""
import hashlib, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_EAX, UC_X86_REG_EIP, UC_X86_REG_FPCW
root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image) + 4095) & ~4095)
u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x40000)
u.mem_map(0x5000000, 4096)
game, cells, pointers, matrix, scale, stack = 0x3000000, 0x3020000, 0x3021000, 0x3022000, 0x3023000, 0x303f000
pack = lambda *v: struct.pack('<' + 'I' * len(v), *(x & 0xffffffff for x in v))
floats = lambda v: struct.pack('<' + 'f' * len(v), *v)
read = lambda p: struct.unpack('<I', u.mem_read(p, 4))[0]
mouse = [0, 0]
visits = 0
def host_input(u, address, size, user):
    global visits
    sp = u.reg_read(UC_X86_REG_ESP)
    assert read(sp + 4) == 0
    u.mem_write(read(sp + 8), pack(0, *mouse, 0, 0, 0))
    visits += 1
    u.reg_write(UC_X86_REG_EAX, 0)
    u.reg_write(UC_X86_REG_EIP, read(sp))
    u.reg_write(UC_X86_REG_ESP, sp + 12)
u.hook_add(UC_HOOK_CODE, host_input, begin=0x801bae, end=0x801bae)
rng = random.Random(0x200443)
out = bytearray(pack(64))
for case in range(64):
    u.mem_write(game, bytes(0x16000))
    count = case % 3
    for index in range(count):
        transform = [1.,0.,0.,0., 0.,1.,0.,0., 0.,0.,1.,0., float(index * 2),0.,0.,1.]
        u.mem_write(matrix, floats(transform))
        u.mem_write(scale, floats([8., 8., 8., 0.]))
        cell = cells + index * 0x400
        u.mem_write(cell, bytes(0x2a4))
        u.mem_write(stack, pack(0x5000000, matrix, scale, 0))
        u.reg_write(UC_X86_REG_ESP, stack)
        u.reg_write(UC_X86_REG_ECX, cell)
        u.reg_write(UC_X86_REG_FPCW, 0x143f)
        u.emu_start(0x81c8bb, 0x5000000, count=100000)
        assert u.reg_read(UC_X86_REG_EIP) == 0x5000000
        u.mem_write(pointers + index * 4, pack(cell))
    u.mem_write(game + 4, pack(pointers, count))
    position = [rng.uniform(-6., 6.), rng.uniform(-3., 3.), rng.uniform(-3., 3.), float(case % 2)]
    if case < 4: position = [[0.,0.,0.,1.], [0.,1.75,-21.,0.], [-0.,-0.,-0.,1.], [4.,0.,0.,1.]][case]
    direction, look = rng.uniform(-3., 3.), rng.uniform(-1., 1.)
    mouse = [rng.randrange(-100000, 100000), rng.randrange(-100000, 100000)]
    # Fields SetPlayer must preserve, with nonzero gameplay timers/control.
    u.mem_write(game + 0x15354, floats([.2, .3, -.1]) + pack(7, 1))
    u.mem_write(game + 0x15510, floats([.4]) + pack(3) + floats([.5]))
    old_matrix = [1.,0.,0.,0., 0.,1.,0.,0., 0.,0.,1.,0., float(case),2.,3.,1.]
    u.mem_write(game + 0x15308, floats(old_matrix))
    u.mem_write(stack, pack(0x5000000) + floats(position + [direction, look]))
    u.reg_write(UC_X86_REG_ESP, stack)
    u.reg_write(UC_X86_REG_ECX, game)
    u.reg_write(UC_X86_REG_FPCW, 0x143f)
    visits = 0
    u.emu_start(0x81cb71, 0x5000000, count=100000)
    assert u.reg_read(UC_X86_REG_EIP) == 0x5000000 and u.reg_read(UC_X86_REG_ESP) == stack + 28
    assert visits == 1
    out.extend(pack(count) + floats(position + [direction, look]) + pack(*mouse) + floats(old_matrix))
    out.extend(u.mem_read(game + 0x14948, 28))  # position, look, direction, cell
    out.extend(u.mem_read(game + 0x15374, 28))  # mouse, last camera y, camera pos
    out.extend(u.mem_read(game + 0x15354, 20))
    out.extend(u.mem_read(game + 0x15510, 12))
    out.extend(u.mem_read(game + 0x15308, 64))
    body = game + 0x149fc
    out.extend(u.mem_read(body, 0x348))
    hit = body + 0x348
    out.extend(u.mem_read(hit + 0x100, 4))
    out.extend(u.mem_read(hit + 0xf0, 4))
    out.extend(u.mem_read(hit + 0x20, 6 * 16))
    out.extend(u.mem_read(hit + 0xe0, 16))
    out.extend(u.mem_read(hit + 0x13c, 6 * 16))
    out.extend(u.mem_read(hit + 0x1a4, 4 * 64))
    # Compare the runtime scene-cell bridge against native transformed planes.
    u.mem_write(stack, pack(0x5000000, hit + 0x1e4))
    u.reg_write(UC_X86_REG_ESP, stack)
    u.reg_write(UC_X86_REG_ECX, hit)
    u.emu_start(0x81a880, 0x5000000, count=30000)
    assert u.reg_read(UC_X86_REG_EIP) == 0x5000000
    out.extend(u.mem_read(hit + 0x20, 6 * 16))
    out.extend(u.mem_read(hit + 0xe0, 16))
p = root / 'tests/fixtures/beta-player-placement.bin'
p.write_bytes(out)
print(len(out), hashlib.sha256(out).hexdigest())
