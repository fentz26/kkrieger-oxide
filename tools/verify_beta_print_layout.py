#!/usr/bin/env python3
"""Capture released Print's five-word ABI; verify native glyph vertices are planar."""
import hashlib
import pathlib
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_EAX, UC_X86_REG_EDI, UC_X86_REG_ECX, UC_X86_REG_EIP
root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image)+4095)&~4095)
u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x40000)
pack = lambda *v: struct.pack('<'+'I'*len(v), *v)
read = lambda p: struct.unpack('<I', u.mem_read(p, 4))[0]
stack, info, env = 0x3030000, 0x3000000, 0x3010000
vertices = []
def hook(u, address, size, user):
    esp = u.reg_read(UC_X86_REG_ESP)
    if address == 0x80d472:
        u.reg_write(UC_X86_REG_EAX, 0)
        u.reg_write(UC_X86_REG_EIP, read(esp))
        u.reg_write(UC_X86_REG_ESP, esp+8)
    elif address == 0x80d4f1:
        vertices.append(struct.unpack('<5f', u.mem_read(esp+4, 20)))
        u.reg_write(UC_X86_REG_EIP, read(esp))
        u.reg_write(UC_X86_REG_ESP, esp+32)
u.hook_add(UC_HOOK_CODE, hook)
output = bytearray(pack(64))
metrics = bytearray(pack(64))
for case in range(64):
    flags = case*17
    values = [(case+1)*.03125, (case+2)*.0625, -.125, .25]
    words = struct.unpack('<4I', struct.pack('<4f', *values))
    u.reg_write(UC_X86_REG_ESP, stack)
    u.mem_write(stack, pack(0, 0, env, flags, *words, 0xdeadbeef))
    u.emu_start(0x80d701, 0x80d75a, count=1000)
    ebp = u.reg_read(UC_X86_REG_EBP)
    assert u.reg_read(UC_X86_REG_EDI) == flags
    captured = bytes(u.mem_read(ebp-0x2c, 16))
    assert captured == struct.pack('<4I', *words)
    output.extend(pack(flags)); output.extend(captured)
    # Native glyph method reads 16-byte UV rectangles and sends literal zero z.
    u.mem_write(info, pack(env, words[0], words[1], words[2], words[3], 0x3020000, 0x3021000, 0, 4096, 0))
    uv = [(.125, .25, .5, .75), (0., .25, 1., .25), (-.25, .75, .125, .5), (.5, .25, .125, .75)][case%4]
    u.mem_write(env+(0x14f+65)*16, struct.pack('<4f', *uv))
    u.mem_write(stack, pack(0x303f000, 0, 0, 65))
    u.reg_write(UC_X86_REG_ESP, stack); u.reg_write(UC_X86_REG_ECX, info)
    vertices.clear(); u.emu_start(0x80d53f, 0x303f000, count=10000)
    assert len(vertices) == (4 if uv[3] > uv[1] else 0) and all(v[2] == 0 for v in vertices)
    first = list(vertices)
    # Capture the actual native Width return through an x87 store.
    u.mem_write(stack, pack(0x303f100, 65))
    u.mem_write(0x303f100, b'\xd9\x1d'+pack(0x3022000))
    u.reg_write(UC_X86_REG_ESP, stack); u.reg_write(UC_X86_REG_ECX, info)
    u.emu_start(0x80d641, 0x303f106, count=1000)
    width_bytes = bytes(u.mem_read(0x3022000, 4))
    width = struct.unpack('<f', width_bytes)[0]
    advance = struct.unpack('<I', struct.pack('<f', width+values[2]))[0]
    u.mem_write(stack, pack(0x303f000, advance, 0, 65))
    u.reg_write(UC_X86_REG_ESP, stack); u.reg_write(UC_X86_REG_ECX, info)
    vertices.clear(); u.emu_start(0x80d53f, 0x303f000, count=10000)
    metrics.extend(struct.pack('<8f', *values, *uv))
    metrics.extend(width_bytes)
    metrics.extend(pack(len(first)+len(vertices)))
    for vertex in first+vertices:
        metrics.extend(struct.pack('<5f', *vertex))
    metrics.extend(bytes((8-len(first)-len(vertices))*20))
p = root / 'tests/fixtures/beta-print-layout.bin'
p.write_bytes(output)
print(len(output), hashlib.sha256(output).hexdigest())

p = root / 'tests/fixtures/beta-print-metrics.bin'
p.write_bytes(metrics)
print(len(metrics), hashlib.sha256(metrics).hexdigest())
