#!/usr/bin/env python3
"""Original modulated-delay Set, stereo channel and mono auxiliary rendering."""
import struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import ROOT,voice_code
code,symbols,fields=voice_code(sections=[('struc syVModDel','global _COMPRESSOR_')]);assert fields['syWModDel.size']==52
address=0x1000000;workspace=0x2000100;parameters=0x2001000;left=0x2100000;right=0x2120000
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(64)+word(6));seed=1234
for c in range(64):
 length=[16,2048,32768][c%3];rate=[22050,44100,48000,96000][c%4];aux=c%2;count=[1,7,64,128,280][c%5]
 p=[float([0,32,64,96,127,128][c%6]),float(16+c*7%97),float(1+c*3%127),float(1+c*11%127),float(c*5%129),float(c*7%129),float(c*13%129)]
 out+=word(length)+word(rate)+word(aux)+word(count)+b''.join(real(v) for v in p)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(0x2000000,0x10000);e.mem_map(left,0x40000);e.mem_write(address,code);e.mem_write(workspace,word(left)+word(right)+word(length-1));e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)));e.emu_start(symbols[name],address+len(code))
 e.reg_write(UC_X86_REG_EAX,rate);call('calcNewSampleRate');call('syModDelInit');e.mem_write(parameters,b''.join(real(v) for v in p));e.reg_write(UC_X86_REG_ESI,parameters);call('syModDelSet');out+=e.mem_read(workspace+8,44)
 # Seed live delay histories, including short rings that wrap repeatedly.
 out+=word(seed)
 for buffer in [left,right]:
  data=bytearray()
  for i in range(length):
   seed=(seed*1664525+1013904223)&0xffffffff;data+=real(float((seed>>16)%1024-512)/512)
  e.mem_write(buffer,bytes(data))
 for block in range(6):
  data=bytearray()
  for i in range(count*3):
   seed=(seed*1664525+1013904223)&0xffffffff;data+=real(float((seed>>16)%1024-512)/512)
  out+=data;e.mem_write(symbols['chanbuf'],bytes(data[:count*8]));e.mem_write(symbols['aux2buf'],bytes(data[count*8:]));e.reg_write(UC_X86_REG_EDI,symbols['chanbuf']);e.reg_write(UC_X86_REG_ECX,count);call('syModDelRenderAux2Main' if aux else 'syModDelRenderChan');out+=e.mem_read(workspace+8,44)+e.mem_read(symbols['chanbuf'],count*8)
 # All history words are compared after the six persistent blocks.
 changed=min(length,count*6)
 out+=e.mem_read(left,changed*4)+e.mem_read(right,changed*4)
path=ROOT/'tests/fixtures/synth-delay.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 64 setups, 384 persistent delay blocks; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
