#!/usr/bin/env python3
"""Compare the complete original voice, including its actual DSP dependencies."""
import struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import voice_code,ROOT
code,symbols,fields=voice_code();address=0x1000000;workspace=0x2000100;parameters=0x2001000
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(64)+word(6));seed=1234
for c in range(64):
 p=[float(c*11%129),64.+(c%5)*.25]
 for j in range(3):p += [float((c+j)%8),float((c//8+j)%2),64.,64.+(j-1)*7,32.+j*32,32.+j*16]
 for j in range(2):p += [float((c+j)%6),80.+j*13+c%16,float(32+c*7%80)]
 p += [float(c%4),63.5+(c%5)*.25,float(c%10),float(32+4*(c%7)),float(64+c%16),float(63+c%3)]
 for j in range(2):p += [float(20+c%40),64.,80.,64.,64.,100.]
 for j in range(2):p += [float((c+j)%8),float(c%2),float((c//2)%2),48.+32*j,float(c*11%128),float(c%3),float(8+c%8)]
 p += [float(c%2)];assert len(p)==59
 seeds=[1234+c*17+j*101 for j in range(5)]
 out+=b''.join(real(v) for v in p)+b''.join(word(v) for v in seeds)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(0x2000000,0x10000);e.mem_write(address,code);e.mem_write(parameters,b''.join(real(v) for v in p));e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)));
  try:e.emu_start(symbols[name],address+len(code))
  except Exception:
   print(c,name,hex(e.reg_read(UC_X86_REG_EIP)), sorted((a,n) for n,a in symbols.items() if a<=e.reg_read(UC_X86_REG_EIP))[-3:]);raise
 e.reg_write(UC_X86_REG_EAX,44100);call('calcNewSampleRate');call('syV2Init')
 for j,offset in enumerate([76,136,196,384,428]):e.mem_write(workspace+offset,word(seeds[j]))
 e.reg_write(UC_X86_REG_ESI,parameters);call('syV2Set');out+=e.mem_read(workspace,552)
 for frame in range(6):
  count=[1,7,64,128,280][(c+frame)%5];note=48+c%24+(2 if frame==3 else 0);velocity=80+c%48
  if frame in (0,3):e.reg_write(UC_X86_REG_EAX,note);e.reg_write(UC_X86_REG_EBX,velocity);call('syV2NoteOn')
  elif frame in (2,5):call('syV2NoteOff')
  call('syV2Tick');out+=word(count)
  for name in ['chanbuf','auxabuf','auxbbuf']:
   data=bytearray()
   for i in range(count*2):
    seed=(seed*1664525+1013904223)&0xffffffff;data+=real(float((seed>>16)%1024-512)/512)
   out+=data;e.mem_write(symbols[name],bytes(data))
  e.reg_write(UC_X86_REG_ECX,count);call('syV2Render');out+=e.mem_read(workspace,552)+e.mem_read(symbols['chanbuf'],count*8)
path=ROOT/'tests/fixtures/synth-voice.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 64 voices, 384 render blocks; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
