#!/usr/bin/env python3
"""Original complete channel: receive, compressor, bass, routed FX and all sends."""
import struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import ROOT,voice_code
sections=[('struc syVBoost','global _MODDEL_'),('struc syVModDel','global _COMPRESSOR_'),('struc syVComp',';#'),('struc syVChan','storeChanValues:')]
code,symbols,fields=voice_code(sections=sections);assert fields['syVChan.size']==116
address=0x1000000;workspace=0x2000100;parameters=0x2030000;left=0x2100000;right=0x2102000
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
word_count=fields['syWChan.size']//4
out=bytearray(word(32)+word(4)+word(word_count));seed=1234
for c in range(32):
 count=[1,7,64,128,280][c%5];rate=[22050,44100,48000,96000][c%4]
 p=[float(32+c*3%97),float(c*7%129),float(c*11%129),float(c*13%129),float(c*17%129),float(c*19%129),float(c*23%129),float(c%2),float(c*5%129),float(c%10),float(32+c%32),float(64+c%32),64.,float([32,64,96,127][c%4]),float(32+c%64),float(1+c%127),float(1+c*3%127),float(c*7%129),float(c*11%129),float(c*13%129),float(c%3),float((c//3)%2),float((c//6)%2),float(c%3),float(64+c%32),float(64+c%32),float(32+c%32),float(32+c%64),64.]
 out+=word(count)+word(rate)+b''.join(real(v) for v in p)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(0x2000000,0x40000);e.mem_map(left,0x4000);e.mem_write(address,code);e.mem_write(parameters,b''.join(real(v) for v in p));e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 e.mem_write(workspace+fields['syWChan.chrw'],word(left)+word(right)+word(2047))
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x203f000);e.mem_write(0x203f000,word(address+len(code)));e.emu_start(symbols[name],address+len(code))
 e.reg_write(UC_X86_REG_EAX,rate);call('calcNewSampleRate');call('syChanInit');e.reg_write(UC_X86_REG_ESI,parameters);call('syChanSet')
 for block in range(4):
  for name,channels in [('chanbuf',2),('mixbuf',2),('aux1buf',1),('aux2buf',1),('auxabuf',2),('auxbbuf',2)]:
   data=bytearray()
   for i in range(count*channels):
    seed=(seed*1664525+1013904223)&0xffffffff;data+=real(float((seed>>16)%1024-512)/512)
   out+=data;e.mem_write(symbols[name],bytes(data))
  e.reg_write(UC_X86_REG_ECX,count);call('syChanProcess')
  for name,channels in [('chanbuf',2),('mixbuf',2),('aux1buf',1),('aux2buf',1),('auxabuf',2),('auxbbuf',2)]:out+=e.mem_read(symbols[name],count*channels*4)
 # Pointer words are represented as zero: Rust owns its delay storage.
 state=bytearray(e.mem_read(workspace,fields['syWChan.size']));offset=fields['syWChan.chrw'];state[offset:offset+8]=bytes(8)
 words=struct.unpack('<'+str(word_count)+'I',state);nonzero=[(i,w) for i,w in enumerate(words) if w];out+=word(len(nonzero))
 for i,w in nonzero:out+=word(i)+word(w)
 out+=e.mem_read(left,8192)+e.mem_read(right,8192)
path=ROOT/'tests/fixtures/synth-channel.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 32 channel setups, 128 complete processing blocks; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
