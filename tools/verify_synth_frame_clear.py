#!/usr/bin/env python3
"""Original RenderBlock buffer clearing, including the EAX auxiliary fill."""
import struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import ROOT,voice_code
source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252');a=source.index('; clear output buffer',source.index('.RenderBlock\n'));a=source.index('mov',a);b=source.index('; process all channels',a)
text='syClear:\npushad\n'+source[a:b]+'\npopad\nret\n'
code,symbols,_=voice_code(extra_text=text,preamble=['todo: .zero 8'])
word=lambda n:struct.pack('<I',n&0xffffffff)
address=0x1000000;out=bytearray(word(6));buffers=[('mixbuf',2),('chanbuf',2),('aux1buf',1),('aux2buf',1),('auxabuf',2),('auxbbuf',2)]
for n in [0,1,7,64,128,280]:
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_write(address,code);e.mem_map(0x2000000,0x10000)
 for name,channels in buffers:e.mem_write(symbols[name],word(0x3f000000)*280*channels)
 e.reg_write(UC_X86_REG_ECX,n);e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)));e.emu_start(symbols['syClear'],address+len(code));out+=word(n)
 for name,channels in buffers:out+=e.mem_read(symbols[name],280*channels*4)
path=ROOT/'tests/fixtures/synth-frame-clear.bin';path.write_bytes(out)
print(f'{len(out)} bytes; six original frame clears; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
