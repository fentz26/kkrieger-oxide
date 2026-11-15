#!/usr/bin/env python3
"""Read-only evidence that the released beta contains recursive portal traversal.

This is a presence/structure check, not a released-executable parity claim.
Requires Capstone in the reference-only Python environment; executes no game code.
"""
import hashlib,json,pathlib,struct
import capstone
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
root=pathlib.Path(__file__).resolve().parent.parent
ref=root/'.reference'
exe=(ref/'pno0001.exe').read_bytes()
assert hashlib.sha256(exe).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
pe=struct.unpack_from('<I',exe,0x3c)[0];base=struct.unpack_from('<I',exe,pe+24+28)[0]
image=(ref/'beta-image.bin').read_bytes();entry=0x81830f;end=0x81859c;table=bytes([0,1,1,5,5,4,4,0,2,3,3,7,7,6,6,2,0,2,1,3,5,7,4,6])
assert image.count(table)==1
table_address=base+image.index(table)
code=image[entry-base:end-base]
cs=Cs(CS_ARCH_X86,CS_MODE_32)
instructions=list(cs.disasm(code,entry))
assert instructions[0].mnemonic=='push' and instructions[0].op_str=='ebp'
assert any(i.mnemonic=='cmp' and i.op_str=='eax, 0x100' for i in instructions)
assert any(i.mnemonic=='mov' and i.op_str==f'edi, 0x{table_address+1:x}' for i in instructions)
assert any(i.mnemonic=='call' and i.op_str==f'0x{entry:x}' for i in instructions)
assert instructions[-1].mnemonic=='ret' and instructions[-1].op_str=='0xc'
callers=[base+i for i in range(len(image)-5) if image[i]==0xe8 and base+i+5+struct.unpack_from('<i',image,i+1)[0]==entry]
assert 0x81993e in callers
report={'executable_sha256':hashlib.sha256(exe).hexdigest(),'capstone_version':capstone.__version__,'entry':hex(entry),'end':hex(end),'code_sha256':hashlib.sha256(code).hexdigest(),'cube_edge_table':hex(table_address),'direct_calls':[hex(i) for i in callers],'scope':'Confirms matching traversal structure is present; does not prove complete beta/source behavior parity.'}
(ref/'beta-portal-evidence.json').write_text(json.dumps(report,indent=2)+'\n')
(ref/'beta-portal-routine.txt').write_text('\n'.join(f'{i.address:08x} {i.mnemonic:12} {i.op_str}' for i in instructions)+'\n')
print(json.dumps(report,indent=2))
