#!/usr/bin/env python3
"""Execute the source helper's x87 fsincos and single-precision store sequence.

Register-only argument setup replaces the C++ wrapper; rounding is explicitly
nearest, matching the portable source oracle rather than released game mode.
"""
import hashlib, pathlib, random, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_ESP, UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_types.hpp').read_text(encoding='cp1252')
start=source.index('__forceinline void sFSinCos(')
helper=source[start:source.index('\n}',start)+2]
assert 'fsincos;' in helper and helper.count('fstp dword ptr')==2
# fld [eax]; fsincos; fstp [ecx] (cos); fstp [edx] (sin); ret
code=bytes.fromhex('d900d9fbd919d91ac3')
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x1000000,4096);u.mem_write(0x1000000,code);u.mem_map(0x2000000,4096)
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
rng=random.Random(0x6673696e)
angles=[struct.unpack('<f',struct.pack('<I',0x3e3fa31d))[0],0.,-0.]+[rng.uniform(-6.25,6.25) for _ in range(125)]
out=bytearray(pack(len(angles)))
for angle in angles:
    data=struct.pack('<f',angle);u.mem_write(0x2000000,data);u.mem_write(0x2000f00,pack(0x1000000+len(code)))
    for reg,value in [(UC_X86_REG_EAX,0x2000000),(UC_X86_REG_ECX,0x2000004),(UC_X86_REG_EDX,0x2000008),(UC_X86_REG_ESP,0x2000f00),(UC_X86_REG_FPCW,0x37f)]:u.reg_write(reg,value)
    u.emu_start(0x1000000,0x1000000+len(code),count=100)
    out.extend(data);out.extend(u.mem_read(0x2000008,4));out.extend(u.mem_read(0x2000004,4))
p=root/'tests/fixtures/euler-fsincos.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
