#!/usr/bin/env python3
"""Verify released PartSystem parameter count and native stack cleanup."""
import hashlib
import pathlib
import struct
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EAX, UC_X86_REG_EIP
root = pathlib.Path(__file__).resolve().parent.parent
image = (root / '.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest() == '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base = 0x7c0000
code = list(Cs(CS_ARCH_X86, CS_MODE_32).disasm(image[0x80dc14-base:0x80e106-base], 0x80dc14))
assert code[-1].mnemonic == 'ret' and code[-1].op_str == '0x34' # op, env, eleven words
# Highest incoming parameter offset is gravity at ebp+0x38; no later flags word.
assert not any('[ebp + 0x3c]' in i.op_str for i in code)
u = Uc(UC_ARCH_X86, UC_MODE_32)
u.mem_map(base, (len(image)+4095)&~4095); u.mem_write(base, image)
u.mem_map(0x3000000, 0x40000)
pack = lambda *v: struct.pack('<'+'I'*len(v), *v)
read = lambda p: struct.unpack('<I', u.mem_read(p, 4))[0]
def hook(u, address, size, user):
    if address == 0x80d472: # Only replace material lookup, returning no material.
        esp = u.reg_read(UC_X86_REG_ESP)
        u.reg_write(UC_X86_REG_EAX, 0)
        u.reg_write(UC_X86_REG_EIP, read(esp)); u.reg_write(UC_X86_REG_ESP, esp+8)
u.hook_add(UC_HOOK_CODE, hook)
for case in range(64):
    stack, env, stop = 0x3030000, 0x3000000, 0x303f000
    words = [case%16, 128, 0xff112233, 0xff445566, 0xff778899, 0x3f800000, 0x40000000, 0x40400000, 0x3f000000, 0x3e800000, 0xbe000000]
    u.mem_write(stack, pack(stop, 0, env, *words, 0xdeadbeef))
    u.reg_write(UC_X86_REG_ESP, stack)
    u.emu_start(0x80dc14, stop, count=10000)
    assert u.reg_read(UC_X86_REG_ESP) == stack+56
    assert read(stack+56) == 0xdeadbeef
print('64 released PartSystem no-material calls: eleven words, native stack cleanup verified')
