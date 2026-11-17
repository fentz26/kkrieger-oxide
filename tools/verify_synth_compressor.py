#!/usr/bin/env python3
"""Original compressor setup, level detectors, lookahead and gain processing."""
import struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import ROOT,voice_code
code,symbols,fields=voice_code(sections=[('struc syVComp',';#')]);assert fields['syWComp.dbuf']==64;assert fields['syWComp.rmsbuf']==45664
address=0x1000000;workspace=0x2000100;parameters=0x2030000;audio=0x2031000
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(64)+word(6));seed=1234
for c in range(64):
 count=[1,7,64,128,280][c%5];p=[float(c%3),float((c//3)%2),float((c//6)%2),[0.,.25,2.,10.][c%4],float(32+c*7%96),float(32+c*5%96),float(c*11%128),float(c*13%128),float(48+c*3%33)]
 out+=word(count)+b''.join(real(v) for v in p)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(0x2000000,0x40000);e.mem_write(address,code);e.mem_write(parameters,b''.join(real(v) for v in p));e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x203f000);e.mem_write(0x203f000,word(address+len(code)));e.emu_start(symbols[name],address+len(code))
 call('syCompInit');e.reg_write(UC_X86_REG_ESI,parameters);call('syCompSet')
 # Exercise wraparound immediately, without changing the zero-initialized histories.
 e.mem_write(workspace+52,word(8190));out+=e.mem_read(workspace,64)
 for block in range(6):
  current=p.copy()
  if block in (2,3):current[0]=float((int(p[0])+1)%3)
  elif block==1:current[5]+=0.5
  elif block==5:current[0]=0.
  packed=b''.join(real(v) for v in current);out+=packed;e.mem_write(parameters,packed);e.reg_write(UC_X86_REG_ESI,parameters);call('syCompSet')
  data=bytearray()
  for i in range(count*2):
   seed=(seed*1664525+1013904223)&0xffffffff;data+=real(float((seed>>16)%2048-1024)/512)
  out+=data;e.mem_write(audio,bytes(data));e.reg_write(UC_X86_REG_ESI,audio);e.reg_write(UC_X86_REG_ECX,count);call('syCompRender');out+=e.mem_read(workspace,64)+e.mem_read(audio,count*8)
 words=struct.unpack('<'+str((fields['syWComp.size']-64)//4)+'I',e.mem_read(workspace+64,fields['syWComp.size']-64));changed=[(i,w) for i,w in enumerate(words) if w]
 out+=word(len(changed))
 for i,w in changed:out+=word(i)+word(w)
path=ROOT/'tests/fixtures/synth-compressor.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 64 setups, 384 compressor blocks; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
