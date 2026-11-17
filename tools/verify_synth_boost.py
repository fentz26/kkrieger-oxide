#!/usr/bin/env python3
"""Original bass shelving coefficients and persistent stereo processing."""
import struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import voice_code,ROOT
code,symbols,fields=voice_code(sections=[('struc syVBoost','global _MODDEL_')]);assert fields['syWBoost.size']==56
address=0x1000000;workspace=0x2000100;parameters=0x2001000;audio=0x2002000
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(128)+word(4));seed=1234
for c in range(128):
 rate=[22050,44100,48000,96000][c%4];amount=[0.,.25,.5,.75,1.,32.,64.,96.,127.,128.,256.,-.5][c%12];count=[1,7,64,128,280][c%5]
 out+=word(rate)+real(amount)+word(count)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(0x2000000,0x10000);e.mem_write(address,code);e.mem_write(parameters,real(amount));e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)));e.emu_start(symbols[name],address+len(code))
 e.reg_write(UC_X86_REG_EAX,rate);call('calcNewSampleRate');call('syBoostInit');e.reg_write(UC_X86_REG_ESI,parameters);call('syBoostSet');out+=e.mem_read(workspace,56)
 for block in range(4):
  # Disable then re-enable a running filter, preserving all delay registers.
  value=0. if block==1 else amount
  out+=real(value);e.mem_write(parameters,real(value));e.reg_write(UC_X86_REG_ESI,parameters);call('syBoostSet')
  data=bytearray()
  for i in range(count*2):
   seed=(seed*1664525+1013904223)&0xffffffff;data+=real(float((seed>>16)%2048-1024)/512)
  out+=data;e.mem_write(audio,bytes(data));e.reg_write(UC_X86_REG_ESI,audio);e.reg_write(UC_X86_REG_ECX,count);call('syBoostRender');out+=e.mem_read(workspace,56)+e.mem_read(audio,count*8)
path=ROOT/'tests/fixtures/synth-boost.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 128 setups and 512 stereo blocks; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
