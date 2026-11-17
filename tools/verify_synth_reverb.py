#!/usr/bin/env python3
"""Original six-line-per-side reverb coefficients, persistent output and resets."""
import struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import ROOT,voice_code
code,symbols,fields=voice_code(sections=[('lencl0 \tequ','global _V2CHAN_')]);assert fields['syWReverb.linecl0']==124
address=0x1000000;workspace=0x2000100;parameters=0x2020000
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(64)+word(4)+word(fields['syWReverb.size']//4));seed=1234
names=['cl0','cl1','cl2','cl3','al0','al1','cr0','cr1','cr2','cr3','ar0','ar1']
for c in range(64):
 count=[1,7,64,128,280][c%5];rate=[22050,44100,48000,96000][c%4];p=[float(c*7%129),float(32+c*3%97),float(c*11%129),float(c*13%129)]
 out+=word(count)+word(rate)+b''.join(real(v) for v in p)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(0x2000000,0x30000);e.mem_write(address,code);e.mem_write(parameters,b''.join(real(v) for v in p));e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x202f000);e.mem_write(0x202f000,word(address+len(code)));e.emu_start(symbols[name],address+len(code))
 e.reg_write(UC_X86_REG_EAX,rate);call('calcNewSampleRate');call('syReverbInit');e.reg_write(UC_X86_REG_ESI,parameters);call('syReverbSet');out+=e.mem_read(workspace,36)
 # Random initial history drives every comb/allpass immediately, near ring wrap.
 out+=word(seed)
 data=bytearray()
 for i in range((fields['syWReverb.size']-124)//4):
  seed=(seed*1664525+1013904223)&0xffffffff;data+=real(float((seed>>16)%1024-512)/512)
 e.mem_write(workspace+124,bytes(data))
 for name in names:e.mem_write(workspace+fields['syWReverb.pos'+name],word(fields['len'+name]-1))
 for block in range(4):
  if block==2:call('syReverbReset')
  data=bytearray()
  for i in range(count*3):
   seed=(seed*1664525+1013904223)&0xffffffff;data+=real(float((seed>>16)%1024-512)/512)
  out+=data;e.mem_write(symbols['mixbuf'],bytes(data[:count*8]));e.mem_write(symbols['aux1buf'],bytes(data[count*8:]));e.reg_write(UC_X86_REG_EDI,symbols['mixbuf']);e.reg_write(UC_X86_REG_ECX,count);call('syReverbProcess');out+=e.mem_read(workspace,124)+e.mem_read(symbols['mixbuf'],count*8)
 words=struct.unpack('<'+str((fields['syWReverb.size']-124)//4)+'I',e.mem_read(workspace+124,fields['syWReverb.size']-124));nonzero=[(i,w) for i,w in enumerate(words) if w];out+=word(len(nonzero))
 for i,w in nonzero:out+=word(i)+word(w)
path=ROOT/'tests/fixtures/synth-reverb.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 64 reverb setups, 256 render blocks; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
