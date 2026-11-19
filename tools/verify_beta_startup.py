#!/usr/bin/env python3
"""Run released startup painter/placement sequence with world-building stubs."""
import hashlib,pathlib,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EBP,UC_X86_REG_ESI,UC_X86_REG_ECX,UC_X86_REG_EIP
root=pathlib.Path(__file__).resolve().parent.parent
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x7c0000,(len(image)+4095)&~4095);u.mem_write(0x7c0000,image)
u.mem_map(0x3000000,0x40000)
doc,ops,env,game,stack=0x3000000,0x3001000,0x3003000,0x3010000,0x303f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
trace=[];pose=None;selected=0

def hook(u,address,size,user):
 global pose,selected
 if address not in [0x81e642,0x81ae3b,0x81da41,0x81cb71]:return
 esp=u.reg_read(UC_X86_REG_ESP)
 assert u.reg_read(UC_X86_REG_ECX)==game
 if address==0x81e642:
  selected=read(esp+4);assert read(esp+8)==env;trace.append('painter');return
 if address==0x81ae3b:trace.append('flush');pop=0
 elif address==0x81da41:
  assert read(esp+4)==selected and read(esp+8)==env
  trace.append('phase'+str(read(esp+12)));pop=12
 else:
  pose=bytes(u.mem_read(esp+4,24));trace.append('place');pop=24
 u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+4+pop)
u.hook_add(UC_HOOK_CODE,hook);out=bytearray(pack(32))
for count in range(1,33):
 u.mem_write(doc+0x10,pack(ops,count));u.mem_write(0x14a099c,pack(doc));u.mem_write(0x14a098c,pack(env));u.mem_write(0x14a0998,pack(game))
 u.mem_write(stack-256,bytes([0xa5])*512);u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_EBP,stack+128)
 trace=[];pose=None;u.emu_start(0x81ec5d,0x81ecc3,count=10000)
 assert trace==['painter','flush','phase0','phase1','place'] and pose==bytes(24)
 assert selected==ops+(count-1)*0x98
 out.extend(pack(count,(selected-ops)//0x98));out.extend(pose)
# Execute the actual constructor switch-initialization tail as a separate capture.
u.mem_write(game+0x14fe8,bytes([0xa5])*256);u.reg_write(UC_X86_REG_ESI,game);u.reg_write(UC_X86_REG_ESP,stack)
u.emu_start(0x81c009,0x81c024,count=1000)
out.extend(u.mem_read(game+0x14fe8,256))
# Native main loop resets the IPP manager around root execution and presentation.
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
instructions=Cs(CS_ARCH_X86,CS_MODE_32).disasm(image[0x81e9be-0x7c0000:0x81ea44-0x7c0000],0x81e9be)
assert [i.op_str for i in instructions if i.mnemonic=='call']==['0x81429e','0x818d42','0x8142e1','0x814e2f','0x81429e']
p=root/'tests/fixtures/beta-startup.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
