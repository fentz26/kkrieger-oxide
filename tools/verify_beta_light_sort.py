#!/usr/bin/env python3
"""Execute only the released beta's light-pointer sort/count clamp in Unicorn.

Reference-only verification; no game code is included in the Rust runtime.
"""
import hashlib
import json
import pathlib
import struct
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EIP
root=pathlib.Path(__file__).resolve().parent.parent
ref=root/'.reference'
exe=(ref/'pno0001.exe').read_bytes()
assert hashlib.sha256(exe).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
pe=struct.unpack_from('<I',exe,0x3c)[0];base=struct.unpack_from('<I',exe,pe+24+28)[0]
image=(ref/'beta-image.bin').read_bytes()
start,end=0x8174b5,0x817512
code=image[start-base:end-base]
assert hashlib.sha256(code).hexdigest()=="758b1cc4590ae57f755525145cacd1dd194331d98a128d8792cf978d6b306868"
instructions=list(Cs(CS_ARCH_X86,CS_MODE_32).disasm(code,start))
assert any(i.address==0x817502 and i.mnemonic=='push' and i.op_str=='4' for i in instructions)
assert any(i.address==0x81750d and i.mnemonic=='mov' and i.op_str=='dword ptr [0x149e6a4], eax' for i in instructions)
assert instructions[-1].address+instructions[-1].size==end
report={'executable_sha256':hashlib.sha256(exe).hexdigest(),'entry':hex(start),'end':hex(end),'code_sha256':hashlib.sha256(code).hexdigest(),'count_address':'0x149e6a4','pointer_array':'0x149e1b0','importance_offset':32,'scope':'Released beta light-pointer sort and four-light clamp only; not complete light preparation or rendering parity.'}
(ref/'beta-light-sort-evidence.json').write_text(json.dumps(report,indent=2)+'\n')
(ref/'beta-light-sort-routine.txt').write_text('\n'.join(f'{i.address:08x} {i.mnemonic:12} {i.op_str}' for i in instructions)+'\n')
u=Uc(UC_ARCH_X86,UC_MODE_32)
u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000);u.mem_map(0x4000000,0x10000)
word=lambda v:struct.pack('<I',v&0xffffffff)
data=bytearray(word(128))
lengths=[0,1,2,3,4,5,8,16,32]
for case in range(128):
    n=lengths[case%len(lengths)]
    values=[float(((i*13+case*7)%11)-5) for i in range(n)]
    if case%4==0:values=[1.0]*n
    if case%4==1:values=[float(i) for i in range(n)]
    if case%4==2 and n>=3:values[n//2]=float("nan")
    if case%4==3 and n>=3:values[0]=float("inf");values[-1]=float("-inf")
    data+=word(n)
    u.mem_write(0x149e6a4,word(n))
    for i,value in enumerate(values):
        pointer=0x3000000+i*64
        u.mem_write(0x149e1b0+i*4,word(pointer));u.mem_write(pointer+32,struct.pack('<f',value))
        data+=struct.pack('<f',value)
    u.reg_write(UC_X86_REG_ESP,0x400f000)
    u.emu_start(start,end,count=1000000)
    assert u.reg_read(UC_X86_REG_EIP)==end
    count=struct.unpack('<I',u.mem_read(0x149e6a4,4))[0]
    assert count==min(n,4)
    data+=word(count)
    for i in range(count):
        pointer=struct.unpack('<I',u.mem_read(0x149e1b0+i*4,4))[0]
        data+=word((pointer-0x3000000)//64)
out=root/'tests/fixtures/beta-light-sort.bin';out.write_bytes(data)
print(f'{out}: {len(data)} bytes, 128 released beta light sorts and count clamps')
print(json.dumps(report,indent=2))
