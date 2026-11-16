#!/usr/bin/env python3
"""Released beta light importance/fade instructions with an explicit x87 mode."""
import hashlib,json,pathlib,struct
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EBP,UC_X86_REG_ESI,UC_X86_REG_EIP,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent;ref=root/'.reference'
exe=(ref/'pno0001.exe').read_bytes();assert hashlib.sha256(exe).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
pe=struct.unpack_from('<I',exe,0x3c)[0];base=struct.unpack_from('<I',exe,pe+24+28)[0];image=(ref/'beta-image.bin').read_bytes()
start,end=0x8173d1,0x81744d;code=image[start-base:end-base]
assert hashlib.sha256(code).hexdigest()=="a8aae5d0812ee524dab551a85f7c7a85181f2c8047390ef6811d2e8a144255f7"
cs=Cs(CS_ARCH_X86,CS_MODE_32);instructions=list(cs.disasm(code,start))
assert instructions[0].mnemonic=='fcom' and instructions[-1].address+instructions[-1].size==end
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
for address in [0x3000000,0x4000000,0x5000000]:u.mem_map(address,0x10000)
bootstrap=b'\xd9\x05'+struct.pack('<I',0x3001000)+b'\xe9'+struct.pack('<i',start-(0x5000000+11));u.mem_write(0x5000000,bootstrap)
word=lambda n:struct.pack('<I',n&0xffffffff)
u.mem_write(0x3000014,word(0x3000100))
data=bytearray(word(512));boundaries=[0.,.1,34.999996185302734,35.,35.000003814697266,44.999996185302734,45.,45.000003814697266]
for case in range(512):
    distance=boundaries[case%8] if case<128 else (case-128)*.125
    radius=[.1,1.,2.,32.,.125][(case//8)%5];amplify=[1.,.1,1./255.,2.,.125,17.][(case//40)%6]
    inputs=struct.pack('<fff',distance,radius,amplify);data+=inputs
    u.mem_write(0x3001000,inputs[:4]);u.mem_write(0x3000110,inputs[4:8]);u.mem_write(0x300010c,inputs[8:12])
    u.reg_write(UC_X86_REG_FPCW,0x143f);u.reg_write(UC_X86_REG_ESP,0x400f000);u.reg_write(UC_X86_REG_EBP,0x400f000);u.reg_write(UC_X86_REG_ESI,0x3000000)
    u.emu_start(0x5000000,end,count=1000);assert u.reg_read(UC_X86_REG_EIP)==end
    data+=u.mem_read(0x3000020,4)+u.mem_read(0x300010c,4)
out=root/'tests/fixtures/beta-light-importance.bin';out.write_bytes(data)
report={'executable_sha256':hashlib.sha256(exe).hexdigest(),'entry':hex(start),'end':hex(end),'code_sha256':hashlib.sha256(code).hexdigest(),'x87_control_word':'0x143f','scope':'Explicit-distance light importance/fade block under declared single-precision downward rounding; excludes distance calculation, global FPU-state control-flow proof and complete lighting parity.'}
(ref/'beta-light-importance-evidence.json').write_text(json.dumps(report,indent=2)+'\n');(ref/'beta-light-importance-routine.txt').write_text('\n'.join(f'{i.address:08x} {i.mnemonic:12} {i.op_str}' for i in instructions)+'\n')
print(f'{out}: {len(data)} bytes, 512 released beta importance/fade calculations');print(json.dumps(report,indent=2))
