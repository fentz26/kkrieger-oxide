#!/usr/bin/env python3
"""Original SetGlobals and complete post-channel master processing."""
import struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import ROOT,voice_code
source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('_synthSetGlobals@4\n');a=source.index('pushad',a);b=source.index('; VU retrieval',a);setup='syMasterSet:\n'+source[a:b]
a=source.index('; Reverb/Delay draufrechnen');b=source.index('.renderend',a);render='syMasterRender:\npushad\n'+source[a:b]+'\npopad\nret\n'
preamble=['.set data,0x2000000','.set SYN.globalsstart,0','.set SYN.globalsend,89','.set SYN.rvbparm,0','.set SYN.delparm,16','.set SYN.vlowcut,44','.set SYN.vhighcut,48','.set SYN.cprparm,52','.set SYN.reverb,128','.set SYN.delay,SYN.reverb+syWReverb.size','.set SYN.compr,SYN.delay+syWModDel.size','.set SYN.lcfreq,SYN.compr+syWComp.size','.set SYN.lcbufl,SYN.lcfreq+4','.set SYN.lcbufr,SYN.lcfreq+8','.set SYN.hcfreq,SYN.lcfreq+12','.set SYN.hcbufl,SYN.lcfreq+16','.set SYN.hcbufr,SYN.lcfreq+20','todo: .zero 8']
sections=[('lencl0 \tequ','global _V2CHAN_'),('struc syVModDel','global _COMPRESSOR_'),('struc syVComp',';#')]
code,symbols,fields=voice_code(sections=sections,extra_text=setup+render,preamble=preamble)
address=0x1000000;data=0x2000000;parameters=0x2030000;left=0x2100000;right=0x2120000
reverb=data+128;delay=reverb+fields['syWReverb.size'];compressor=delay+fields['syWModDel.size'];filters=compressor+fields['syWComp.size']
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(32)+word(4)+word((filters-data+24)//4));seed=1234
for c in range(32):
 rate=[22050,44100,48000,96000][c%4];count=[1,7,64,128,280][c%5]
 p=bytes([32+c%64,32+c%64,c*7%129,64+c%64,[32,64,96,127][c%4],32+c%64,1+c%127,1+c*3%127,c*7%129,c*11%129,c*13%129,c*17%128,127 if c%3==0 else c*19%128,c%3,(c//3)%2,(c//6)%2,c%3,64+c%32,64+c%32,32+c%32,32+c%64,64,0])
 out+=word(rate)+word(count)+p
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(data,0x40000);e.mem_map(left,0x40000);e.mem_write(address,code);e.mem_write(parameters,p);e.mem_write(delay,word(left)+word(right)+word(32767));e.reg_write(UC_X86_REG_FPCW,0x7f)
 def call(name,argument=None):
  e.reg_write(UC_X86_REG_ESP,0x203f000);e.mem_write(0x203f000,word(address+len(code))+(word(argument) if argument is not None else b''));e.emu_start(symbols[name],address+len(code))
 e.reg_write(UC_X86_REG_EAX,rate);call('calcNewSampleRate')
 for name,workspace in [('syReverbInit',reverb),('syModDelInit',delay),('syCompInit',compressor)]:e.reg_write(UC_X86_REG_EBP,workspace);call(name)
 call('syMasterSet',parameters)
 for block in range(4):
  for name,channels in [('mixbuf',2),('aux1buf',1),('aux2buf',1)]:
   values=bytearray()
   for i in range(count*channels):
    seed=(seed*1664525+1013904223)&0xffffffff;values+=real(float((seed>>16)%1024-512)/512)
   out+=values;e.mem_write(symbols[name],bytes(values))
  e.mem_write(symbols['todo']+4,word(count));call('syMasterRender');out+=e.mem_read(symbols['mixbuf'],count*8)
 state=bytearray(e.mem_read(data,filters-data+24));state[delay-data:delay-data+8]=bytes(8);words=struct.unpack('<'+str(len(state)//4)+'I',state);nonzero=[(i,w) for i,w in enumerate(words) if w];out+=word(len(nonzero))
 for i,w in nonzero:out+=word(i)+word(w)
 changed=count*4
 out+=e.mem_read(left,changed*4)+e.mem_read(right,changed*4)
path=ROOT/'tests/fixtures/synth-master.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 32 global setups, 128 complete master blocks; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
