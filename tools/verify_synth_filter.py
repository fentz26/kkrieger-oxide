#!/usr/bin/env python3
"""Original filter Init/Set/Render, including stride and bypass copy behavior."""
import pathlib,re,struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_ECX,UC_X86_REG_EBP,UC_X86_REG_ESP,UC_X86_REG_FPCW
from x86_asm import assemble
root=pathlib.Path(__file__).resolve().parents[1]
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('syFltInit:');b=source.index('; Low Frequency Oscillator',a);text=source[a:b]
a=source.index('calcfreq:');b=source.index('pow2:',a);text+='\n'+source[a:b]
for i,name in enumerate(['mode','cutoff','reso']):text=text.replace('syVFlt.'+name,str(i*4))
for i,name in enumerate(['mode','cfreq','res','l','b','step']):text=text.replace('syWFlt.'+name,str(i*4))
lines=[];scope=''
for line in text.splitlines():
 line=line.split(';')[0].strip()
 if not line or line.startswith(('%','V2Perf')):continue
 if line.endswith(':') and not line.startswith('.'):scope=line[:-1]
 line=re.sub(r'(?<![\w.])\.(\w+)',lambda m:scope+'.'+m[1],line)
 line=re.sub(r'\b(dword|byte)\s+\[',r'\1 ptr [',line)
 line=re.sub(r'\bst([0-7])\b',r'st(\1)',line)
 if re.search(r'\bdd\s',line):
  label,data=re.split(r'\bdd\s+',line,maxsplit=1);line=(label.strip()+': ' if label.strip() else '')+'.long '+data.rstrip(',')
 if re.fullmatch(r'syFltRender\.\w+',line):line+=':'
 lines.append(line)
for name in ['fci128','fc10','SRfclinfreq','dcoffset']:
 value=re.search(r'^'+name+r'\s+dd\s+([^;\r\n]+)',source,re.M).group(1).strip();bits=int.from_bytes(struct.pack('<f',float(value)),'little');lines.append(f'{name}: .long {bits}')
address=0x1000000;workspace=0x2000100;parameters=0x2000200;input_address=0x2001000;output_address=0x2003000
code,symbols=assemble('\n'.join(lines),address)
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(128)+word(3));seed=1234
for c in range(128):
 values=[float(c%8),float(c*11%128)+.25,float(c*23%128)+.5];scale=[1.,.5,.25,.91875][(c//2)%4];stride=1+c%2;count=[1,2,5,16,128,280,3][c%7];alias=c%3==0;length=count*stride
 out+=b''.join(real(v) for v in values)+real(scale)+word(stride*4)+word(count)+word(alias)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,4096);e.mem_map(0x2000000,0x10000);e.mem_write(address,code);e.mem_write(parameters,b''.join(real(v) for v in values));e.mem_write(symbols['SRfclinfreq'],real(scale));e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)));e.emu_start(symbols[name],address+len(code))
 call('syFltInit');e.mem_write(workspace+20,word(stride*4));e.reg_write(UC_X86_REG_ESI,parameters);call('syFltSet');out+=e.mem_read(workspace,24)
 for block in range(3):
  samples=[]
  for i in range(length):
   seed=(seed*1664525+1013904223)&0xffffffff
   samples.append((1. if i==0 else 0.) if block==0 else float((seed>>16)%1024-512)/512 if block==1 else .125)
  data=b''.join(real(v) for v in samples);out+=data;e.mem_write(input_address,data);e.mem_write(output_address,real(-777)*length)
  e.reg_write(UC_X86_REG_ESI,input_address);e.reg_write(UC_X86_REG_EDI,input_address if alias else output_address);e.reg_write(UC_X86_REG_ECX,count);call('syFltRender')
  out+=e.mem_read(workspace,24)+e.mem_read(input_address if alias else output_address,length*4)
path=root/'tests/fixtures/synth-filter.bin';path.write_bytes(out)
print(f'{path}: {len(out)} bytes, 128 original filter conversions and 384 persistent render blocks; code SHA256 {hashlib.sha256(code).hexdigest()}')
