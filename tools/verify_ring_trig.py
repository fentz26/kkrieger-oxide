#!/usr/bin/env python3
"""Capture source Ring's double-return trig helpers before its float store.

Execute x87 fsin/fcos, multiply by float radius and store to float, using
nearest rounding. This is a source-helper check, not the beta round-down mode.
"""
import hashlib
import pathlib
import random
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ESP, UC_X86_REG_FPCW
root = pathlib.Path(__file__).resolve().parent.parent
source = (root / '.reference/fr_public/werkkzeug3_kkrieger/_types.hpp').read_text(encoding='cp1252')
assert 'sF64 sFSin(sF64 f)' in source and 'sF64 sFCos(sF64 f)' in source
# fldz; fadd [eax] (phase + zero first-vertex angle); fsin;
# fmul [eax+4]; fstp [eax+8]; repeat for negative cosine; ret
code = bytes.fromhex('d9eed800d9fed84804d95808d9eed800d9ffd9e0d84804d9580cc3')
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(0x1000000, 4096)
u.mem_write(0x1000000, code)
u.mem_map(0x2000000, 4096)
rng = random.Random(0x72696e67)
angles = [0., -0., struct.unpack('<f', struct.pack('<I', 0x3e3fa31d))[0]]
angles += [rng.uniform(-8, 8) for _ in range(509)]
out = bytearray(struct.pack('<I', len(angles)))
for i, angle in enumerate(angles):
    radius = [0., 1., 0.5, 0.33984375, -1., rng.uniform(-4, 4)][i % 6]
    data = struct.pack('<2f', angle, radius)
    u.mem_write(0x2000000, data)
    u.mem_write(0x2000f00, struct.pack('<I', 0x1000000 + len(code)))
    u.reg_write(UC_X86_REG_EAX, 0x2000000)
    u.reg_write(UC_X86_REG_ESP, 0x2000f00)
    u.reg_write(UC_X86_REG_FPCW, 0x37f)
    u.emu_start(0x1000000, 0x1000000 + len(code), count=100)
    out.extend(data)
    out.extend(u.mem_read(0x2000008, 8))
p = root / 'tests/fixtures/ring-trig.bin'
p.write_bytes(out)
print(len(out), hashlib.sha256(out).hexdigest())
