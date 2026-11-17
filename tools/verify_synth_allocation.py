#!/usr/bin/env python3
"""Original note-on voice selection and allocation-table updates."""
import struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import ROOT,voice_code
source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('movzx eax, byte [data + SYN.chans + 8*ecx]',source.index('ProcessNoteOn:'));b=source.index('call\t\tstoreV2Values',a)
text='syAllocate:\n'+source[a:b]+'\nret\n'
preamble=['.set data,0x2000000','.set SYN.patchmap,0','.set SYN.curalloc,8','.set SYN.chanmap,16','.set SYN.allocpos,144','.set SYN.chans,272','.set SYN.voicemap,400','.set SYN.voicesw,8020','.set POLY,32','.set v2sound.maxpoly,88']
code,symbols,fields=voice_code(extra_text=text,preamble=preamble)
word=lambda n:struct.pack('<I',n&0xffffffff)
address=0x1000000;data=0x2000000;bank=0x2010000;seed=1234;out=bytearray(word(512))
for c in range(512):
 def random():
  global seed
  seed=(seed*1664525+1013904223)&0xffffffff;return seed
 channel=c%16;limit=[1,2,4,8,16,32][c%6];counter=0xffffffff if c%17==0 else 1000+c
 channels=[-1 if c%3==0 and random()%3==0 else random()%16 for _ in range(32)]
 ages=[random()%counter for _ in range(32)];gates=[random()%3 for _ in range(32)];latest=[random()%32 for _ in range(16)]
 if c%5==0:channels=[channel]*32
 if c%7==0:gates=[1]*32
 if c%11==0:gates=[0]*32
 if c%13==0:ages=[5]*32
 out+=word(channel)+word(limit)+word(counter)+b''.join(word(v) for v in channels+ages+gates+latest)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(data,0x20000);e.mem_write(address,code);e.mem_write(data,word(bank));e.mem_write(bank,word(512));e.mem_write(bank+512+88,bytes([limit]));e.mem_write(data+8,word(counter));e.mem_write(data+16,b''.join(word(v) for v in channels));e.mem_write(data+144,b''.join(word(v) for v in ages));e.mem_write(data+400,b''.join(word(v) for v in latest))
 for i,gate in enumerate(gates):e.mem_write(data+8020+i*552+8,word(gate))
 e.reg_write(UC_X86_REG_ECX,channel);e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)));e.emu_start(symbols['syAllocate'],address+len(code))
 out+=word(e.reg_read(UC_X86_REG_EDX))+e.mem_read(data+8,4)+e.mem_read(data+16,256)+e.mem_read(data+400,64)
path=ROOT/'tests/fixtures/synth-allocation.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 512 original voice allocation states; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
