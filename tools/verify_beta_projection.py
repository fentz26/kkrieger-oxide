#!/usr/bin/env python3
"""Capture the released sMaterialEnv::MakeProjectionMatrix (0x8022a7).

The complete routine runs under PC=24/RC=down with the system viewport read
through the original global at 0x83f868. Output words are pre-filled with
sentinels, so modes that leave the destination untouched are recorded as such.
"""
import hashlib, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_EIP, UC_X86_REG_FPCW
root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
assert hashlib.sha256(image[0x8022a7 - 0x7c0000:0x8023a5 - 0x7c0000]).hexdigest() == '8907e0fb336789682786a431ca296b2062bfa1203e661c581d0a1262e6d71f9d'
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x7c0000, (len(image) + 4095) & ~4095)
u.mem_write(0x7c0000, image)
u.mem_map(0x3000000, 0x20000)
system, env, out_matrix, stack, done = 0x3000000, 0x3001000, 0x3002000, 0x301f000, 0x301ff00
pack = lambda *v: struct.pack('<' + 'I' * len(v), *(v & 0xffffffff for v in v))
word = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]
u.mem_write(0x83f868, pack(system))
rng = random.Random(0x8022a7)
sizes = [640, 480, 800, 600, 1024, 768, 1, 3, 7, 0, -1, -640, 1920, 1080, 4096, 2147483647]
cases = 512
out = bytearray(pack(cases))
for case in range(cases):
    mode = [0, 1, 2, 3, -1, 0, 2, 0][case % 8]
    fields = [word(rng.uniform(-4., 4.)) for _ in range(6)]
    if case % 16 == 5:
        fields = [0x80000000, 0, 0x7f7fffff, 0xff7fffff, 0x80000001, 1]
    vx, vy = sizes[case % 16], sizes[(case // 16) % 16]
    if case >= 256:
        vx, vy = rng.randrange(1, 5000), rng.randrange(1, 5000)
    u.mem_write(system + 0x204, pack(vx, vy))
    u.mem_write(env + 0x15c, pack(*fields, mode))
    u.mem_write(out_matrix, pack(*(0xc0de0000 + n for n in range(16))))
    u.mem_write(stack, pack(done, out_matrix))
    u.reg_write(UC_X86_REG_ECX, env)
    u.reg_write(UC_X86_REG_ESP, stack)
    u.reg_write(UC_X86_REG_FPCW, 0x143f)
    u.emu_start(0x8022a7, done, count=1000)
    assert u.reg_read(UC_X86_REG_EIP) == done
    assert u.reg_read(UC_X86_REG_ESP) == stack + 8
    out.extend(pack(mode, *fields, vx, vy))
    out.extend(u.mem_read(out_matrix, 64))
p = root / 'tests/fixtures/beta-projection.bin'
p.write_bytes(out)
print(len(out), hashlib.sha256(out).hexdigest())
