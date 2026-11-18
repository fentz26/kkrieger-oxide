#!/usr/bin/env python3
"""Verify released IPP handler identities and parameterless select dispatch."""
import hashlib,json,pathlib,struct
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
root=pathlib.Path(__file__).resolve().parent.parent;ref=root/'.reference'
exe=(ref/'pno0001.exe').read_bytes();assert hashlib.sha256(exe).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(ref/'beta-image.bin').read_bytes();base=0x7c0000
records=struct.pack('<IIIIII',0xea,0x81464a,0x814a0e,0xf0,0x814541,0x814683)
assert image.count(records)==1
select=image[0x814a0e-base:0x814a1e-base]
cs=Cs(CS_ARCH_X86,CS_MODE_32);instructions=list(cs.disasm(select,0x814a0e))
assert [(i.mnemonic,i.op_str) for i in instructions]==[('push','ebp'),('mov','ebp, esp'),('push','dword ptr [ebp + 0xc]'),('mov','ecx, dword ptr [ebp + 8]'),('call','0x818e4a'),('pop','ebp'),('ret','')]
demo=image[0x8151a1-base:0x8151b3-base]
demo_instructions=list(cs.disasm(demo,0x8151a1))
assert [(i.mnemonic,i.op_str) for i in demo_instructions]==[('push','ebp'),('mov','ebp, esp'),('push','dword ptr [ebp + 0xc]'),('mov','ecx, dword ptr [ebp + 8]'),('call','0x818e4a'),('pop','ebp'),('ret','8')]
assert image.count(struct.pack('<III',0xe0,0x815166,0x8151a1))==1
report={'executable_sha256':hashlib.sha256(exe).hexdigest(),'handler_table':hex(base+image.index(records)),'select_entry':'0x814a0e','select_sha256':hashlib.sha256(select).hexdigest(),'scope':'Beta Select delegates only to KOp::ExecInputs, without an authored count or reference adjustment. Viewport signature is structurally different and remains unimplemented.'}
report['scene_wrapper_entry']='0x8151a1'
report['scene_wrapper_sha256']=hashlib.sha256(demo).hexdigest()
(ref/'beta-ipp-evidence.json').write_text(json.dumps(report,indent=2)+'\n')
(ref/'beta-ipp-select.txt').write_text('\n'.join(f'{i.address:08x} {i.mnemonic} {i.op_str}' for i in instructions)+'\n')
print(json.dumps(report,indent=2))
