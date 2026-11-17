#!/usr/bin/env python3
"""Original syEnvSet coefficient conversion with original x87 helpers/constants."""
import pathlib,re,struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBP,UC_X86_REG_ESP,UC_X86_REG_FPCW
from x86_asm import assemble
root=pathlib.Path(__file__).resolve().parents[1]
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('syEnvSet:');b=source.index('; tick',a);text=source[a:b]
a=source.index('calcfreq2:');b=source.index('global calcfreq2',a);text+='\n'+source[a:b]
a=source.index('pow2:');b=source.index('\npow:',a);text+='\n'+source[a:b]
for i,name in enumerate(['ar','dr','sl','sr','rr','vol']):text=text.replace('syVEnv.'+name,str(i*4))
for i,name in enumerate(['out','state','val','atd','dcf','sul','suf','ref','gain']):text=text.replace('syWEnv.'+name,str(i*4))
lines=[]
for line in text.splitlines():
 line=line.split(';')[0].strip()
 if not line:continue
 line=re.sub(r'\b(dword|byte)\s+\[',r'\1 ptr [',line)
 line=re.sub(r'\bst([0-7])\b',r'st(\1)',line)
 lines.append(line)
for name in ['fcattackmul','fcattackadd','fci128','fccfframe','fc64','fcsusmul']:
 value=re.search(r'^'+name+r'\s+dd\s+([^;\r\n]+)',source,re.M).group(1).strip()
 bits=struct.unpack('<I',struct.pack('<f',float(value)))[0];lines.append(f'{name}: .long {bits}')
address=0x1000000;workspace=0x2000100;parameters=0x2000200
code,symbols=assemble('\n'.join(lines),address)
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(512));seed=0x1234
for c in range(512):
 values=[]
 for j in range(6):
  seed=(seed*1664525+1013904223)&0xffffffff
  values.append(float((seed>>16)%128)+(c%16)*.0625)
 if c<8:values=[float([0,1,32,64,127,128,-1,255][c])]*6
 data=b''.join(real(v) for v in values);out+=data
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,4096);e.mem_map(0x2000000,4096);e.mem_write(address,code);e.mem_write(parameters,data)
 initial=real(16)+word(3)+real(64)+real(17)*6;e.mem_write(workspace,initial)
 e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_ESI,parameters);e.reg_write(UC_X86_REG_FPCW,0x7f);e.reg_write(UC_X86_REG_ESP,0x2000800);e.mem_write(0x2000800,word(address+len(code)))
 e.emu_start(symbols['syEnvSet'],address+len(code));out+=e.mem_read(workspace,36)
path=root/'tests/fixtures/synth-envelope-set.bin';path.write_bytes(out)
print(f'{path}: {len(out)} bytes, 512 original x87 envelope parameter conversions; code SHA256 {hashlib.sha256(code).hexdigest()}')
