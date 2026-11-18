#!/usr/bin/env python3
"""Execute the released monster field-copy instructions, without callee stubs."""
import hashlib
import pathlib
import random
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EBP, UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_ESP, UC_X86_REG_FPCW
root = pathlib.Path(__file__).resolve().parent.parent
ref = root / '.reference'
assert hashlib.sha256((ref / 'pno0001.exe').read_bytes()).hexdigest() == 'ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image = (ref / 'beta-image.bin').read_bytes()
base, start, end = 0x7c0000, 0x81d82f, 0x81d85e
assert hashlib.sha256(image[start-base:end-base]).hexdigest() == '40469a9e2318ee06c41e02b5d26a19c35843b50f8d78602e6c781c5aada7b7a5'
assert struct.unpack_from('<I', image, 0x81f2c4-base)[0] == 0x3a83126f
# KOp::Exec pushes this control word then loads it before handler dispatch.
assert image[0x818d53-base:0x818d5b-base] == bytes.fromhex('683f140000d92c24')
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(base, (len(image)+4095)&~4095)
u.mem_write(base, image)
u.mem_map(0x3000000, 0x10000)
actor, words, frame = 0x3000000, 0x3002000, 0x3004000
rng = random.Random(0x200404)
output = bytearray(struct.pack('<I', 256))
for case in range(256):
    # Bounded finite values avoid hardware-specific NaN payload propagation.
    data = [struct.unpack('<I', struct.pack('<f', rng.uniform(-1e5, 1e5)))[0] for _ in range(26)]
    edge = [0, 0x80000000, 0x3f800000, 0xbf800000, 1, 0x80000001,
            0x007fffff, 0x807fffff, 0x00800000, 0x80800000,
            0x7f7fffff, 0xff7fffff]
    if case < len(edge):
        data[6] = edge[case]
    packed = struct.pack('<26I', *data)
    u.mem_write(words, packed)
    u.mem_write(actor, bytes([0xa5])*0x200)
    u.mem_write(frame-8, struct.pack('<I', words))
    u.reg_write(UC_X86_REG_ESP, frame+0x1000)
    u.reg_write(UC_X86_REG_FPCW, 0x143f)
    u.reg_write(UC_X86_REG_EBP, frame)
    u.reg_write(UC_X86_REG_EBX, actor)
    u.reg_write(UC_X86_REG_ESI, words)
    u.emu_start(start, end, count=100)
    output.extend(packed)
    output.extend(u.mem_read(actor+0xbc, 12))
    output.extend(u.mem_read(actor+0xcc, 12))
    output.extend(u.mem_read(actor+0x40, 56))
path = root/'tests/fixtures/beta-monster-copy.bin'
path.write_bytes(output)
print(f'{len(output)} bytes, 256 released field-copy traces, sha256 {hashlib.sha256(output).hexdigest()}')
