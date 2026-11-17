#!/usr/bin/env python3
"""Original assembly envelope state transitions under x86/x87 emulation."""
import pathlib,re,struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_EBP,UC_X86_REG_ESP,UC_X86_REG_FPCW
from x86_asm import assemble
root=pathlib.Path(__file__).resolve().parents[1]
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('syEnvInit:');b=source.index('syEnvSet:',a);init=source[a:b]
a=source.index('syETTab');b=source.index(';  Filter',a);tick=source[a:b]
text=init+'\n'+tick
fields=['out','state','val','atd','dcf','sul','suf','ref','gain']
for i,name in enumerate(fields):text=text.replace('syWEnv.'+name,str(i*4))
text=text.replace('LOWEST','0x39000000')
lines=[]
for line in text.splitlines():
 line=line.split(';')[0].strip()
 if not line:continue
 line=re.sub(r'\b([0-9][0-9a-fA-F]*)h\b',r'0x\1',line)
 line=re.sub(r'(?<![\w.])\.(\w+)',r'syEnvTick.\1',line)
 line=re.sub(r'\b(dword|byte)\s+\[',r'\1 ptr [',line)
 if re.search(r'\bdd\s',line):
  label,data=re.split(r'\bdd\s+',line,maxsplit=1)
  line=(label.strip()+': ' if label.strip() else '')+'.long '+data.rstrip(',')
 elif re.fullmatch(r'[\w.]+',line) and line not in ['pushad','popad','ret']:line+=':'
 lines.append(line)
address=0x1000000;workspace=0x2000100
code,symbols=assemble('\n'.join(lines),address)
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(128)+word(64))
for c in range(128):
 values=[0,c%5, [0.,.0001,1.,64.,128.,256.,-1.][c%7], [0.,.03125,1.,17.,128.][(c//2)%5], [0.,.25,.9,1.,1.01][(c//3)%5], [0.,.0001,1.,32.,127.][(c//4)%5], [0.,.5,1.,2.][(c//5)%4], [0.,.5,.99,1.][(c//6)%4], [0.,.25,1.,2.][(c//7)%4]]
 data=b''.join(word(v) if i==1 else real(v) for i,v in enumerate(values));out+=data
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,4096);e.mem_map(0x2000000,4096);e.mem_write(address,code);e.mem_write(workspace,data);e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 for k in range(64):
  gate=int(k%23<15) if c%2 else int(k<32 or k>=48);out+=word(gate)
  e.reg_write(UC_X86_REG_EAX,gate);e.reg_write(UC_X86_REG_ESP,0x2000800);e.mem_write(0x2000800,word(address+len(code)))
  e.emu_start(symbols['syEnvTick'],address+len(code));out+=e.mem_read(workspace,36)
path=root/'tests/fixtures/synth-envelope.bin';path.write_bytes(out)
print(f'{path}: {len(out)} bytes, 8192 original x87 envelope ticks; code SHA256 {hashlib.sha256(code).hexdigest()}')
