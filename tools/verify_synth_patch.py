#!/usr/bin/env python3
"""Original patch copies and modulation matrices, stopping before DSP Set calls."""
import re,struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import ROOT,voice_code
from x86_asm import assemble
source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
_,_,fields=voice_code()
# Original SYN offsets through chansv, with 32 voices and 16 channels.
# The channel-workspace address is computed but never dereferenced before Set.
defs={'SYN.patchmap':0,'SYN.chanmap':16,'SYN.chans':272,'SYN.voicemap':400,'SYN.voicesv':468,'SYN.voicesw':8020,'SYN.chansv':25684,'SYN.chansw':27540,'syVChan.size':116,'syWChan.size':0,'v2sound.endvdvals':59,'v2sound.chan':59,'v2sound.endcdvals':88,'v2sound.modnum':89,'v2sound.modmatrix':90,'v2mod.source':0,'v2mod.val':1,'v2mod.dest':2}
text=source[source.index('; table for mod sources'):source.index('; Bass, Bass')]+ '\n'+source[source.index('storeChanValues:'):source.index(';#####################################################################################',source.index('storeChanValues:'))]
# Coefficient conversion is verified separately; this fixture observes its input.
text=re.sub(r'call\s+sy(?:V2|Chan)Set','nop',text)
lines=[f'.set {k},{v}' for k,v in (fields|defs).items()];scope=''
for line in text.splitlines():
 line=line.split(';')[0].strip()
 if not line:continue
 if line.endswith(':') and not line.startswith('.'):scope=line[:-1]
 line=re.sub(r'(?<![\w.])\.(\w+)',lambda m:scope+'.'+m[1],line)
 line=re.sub(r'\b([0-9][0-9a-fA-F]*)h\b',r'0x\1',line)
 line=re.sub(r'\b(dword|qword|word|byte)\s+\[',r'\1 ptr [',line)
 line=re.sub(r'\bst([0-7])\b',r'st(\1)',line)
 if re.search(r'\bdd\s',line):
  label,data=re.split(r'\bdd\s+',line,maxsplit=1);line=(label.strip()+': ' if label.strip() else '')+'.long '+data
 elif re.fullmatch(r'\w+\.\w+',line):line+=':'
 lines.append(line)
lines+=['.set data,0x2000000','temp: .zero 64','fc48: .float 48.0','fc64: .float 64.0','fci128: .float 0.0078125']
address=0x1000000;code,symbols=assemble('\n'.join(lines),address)
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(512));seed=1234
for c in range(512):
 def random():
  global seed
  seed=(seed*1664525+1013904223)&0xffffffff;return seed
 base=bytes(random()>>24 for _ in range(88));controls=bytes(random()>>24 for _ in range(7));note=[-2147483648,-64,0,48,60,127,2147483647][c%7];values=[struct.unpack('<f',word(bits))[0] for bits in [0x80000000,0x7fc00123,0xffc00123,0x7f800000,0xff800000]] if c%16==0 else [float((random()>>16)%2048-512)/8 for _ in range(5)]
 mods=bytes(v for i in range(c%32) for v in [(c+i)%256,random()>>24,(c*3+(i%3 if c%4==0 else i)*7)%256]);patch=base+bytes([1,len(mods)//3])+mods
 out+=base+controls+word(note)+b''.join(real(v) for v in values)+word(len(mods)//3)+mods
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(0x2000000,0x20000);e.mem_write(address,code);e.reg_write(UC_X86_REG_FPCW,0x7f)
 channel=c%16;voice=c%32;program=c%128;data=0x2000000;bank=0x2010000;work=data+8020+voice*552
 e.mem_write(data,word(bank));e.mem_write(bank+program*4,word(512));e.mem_write(bank+512,patch);e.mem_write(data+16+voice*4,word(channel));e.mem_write(data+272+channel*8,bytes([program])+controls);e.mem_write(data+400+channel*4,word(voice));e.mem_write(work,word(note)+real(values[0]))
 for offset,v in zip([276,312,348,392],values[1:]):e.mem_write(work+offset,real(v))
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)));e.emu_start(symbols[name],address+len(code))
 e.reg_write(UC_X86_REG_EDX,voice);call('storeV2Values');out+=e.mem_read(data+468+voice*236,236)
 e.reg_write(UC_X86_REG_EBX,channel);call('storeChanValues');out+=e.mem_read(data+25684+channel*116,116)
path=ROOT/'tests/fixtures/synth-patch.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 512 patch matrices; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
