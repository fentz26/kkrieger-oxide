#!/usr/bin/env python3
"""Original complete oscillator renderer, across all eight waveform slots."""
import pathlib,re,struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EDI,UC_X86_REG_ECX,UC_X86_REG_EBP,UC_X86_REG_ESP,UC_X86_REG_FPCW
from x86_asm import assemble
root=pathlib.Path(__file__).resolve().parents[1]
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('oscjtab dd');b=source.index(';  Envelope Generator',a);text=source[a:b]
a=source.index('fastsinrc:');b=source.index('global calcNewSampleRate',a);text+='\n'+source[a:b]
for i,name in enumerate(['mode','ring','cnt','freq','brpt','nffrq','nfres','nseed','gain','gain4','tmp','nfl','nfb','note','pitch']):text=re.sub(r'\bsyWOsc\.'+name+r'\b',str(i*4),text)
text=text.replace('icnoisemul','196314165').replace('icnoiseadd','907633515').replace('cs:','')
lines=[];scope='syOscRender'
for line in text.splitlines():
 line=line.split(';')[0].strip()
 if not line or line.startswith(('section','V2Perf')):continue
 if line.endswith(':') and not line.startswith('.'):scope=line[:-1]
 line=re.sub(r'(?<![\w.])\.(\w+)',lambda m:scope+'.'+m[1],line)
 line=re.sub(r'\b([0-9][0-9a-fA-F]*)h\b',r'0x\1',line)
 line=re.sub(r'\b(dword|qword|byte)\s+\[',r'\1 ptr [',line)
 line=re.sub(r'\bst([0-7])\b',r'st(\1)',line)
 if re.search(r'\bdd\s',line):
  label,data=re.split(r'\bdd\s+',line,maxsplit=1);line=(label.strip()+': ' if label.strip() else '')+'.long '+data.rstrip(',')
 elif re.fullmatch(r'syOscRender\.\w+',line):line+=':'
 lines.append(line)
for name in ['fcpi','fcpi_2','fc1p5pi','fc2pi','fci2','fcgain','fcfmmax','fcsinx3','fcsinx5','fcsinx7']:
 kind,value=re.search(r'^'+name+r'\s+(dd|dq)\s+([^;\r\n]+)',source,re.M).groups();double=kind=='dq';bits=int.from_bytes(struct.pack('<d' if double else '<f',float(value.strip())),'little');lines.append(f'{name}: '+('.quad ' if double else '.long ')+str(bits))
lines.extend(['temp: .zero 64','auxabuf: .zero 2240','auxbbuf: .zero 2240'])
address=0x1000000;workspace=0x2000100;output_address=0x2001000
code,symbols=assemble('\n'.join(lines),address)
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(128)+word(3));seed=1234
for c in range(128):
 count=[1,2,7,128,280][c%5];mode=c%8;ring=(c//8)%4;counter=[0,0xfffffff0,0x10000000,0x7fffffff,0x80000000,0xc0000000][(c//4)%6];frequency=[0,512,0x00100000,0x01000000,0x20000000,0x40000000,0x80000000,0xffffffff][(c//8)%8];breakpoint=[0,0x10000000,0x40000000,0x80000000,0xf0000000,0xffffffff][(c//16)%6];gain=[0.,.25,.5,1.,1.25][(c//4)%5];nseed=(c*196314165+1234)&0xffffffff
 initial=b''.join([word(mode),word(ring),word(counter),word(frequency),word(breakpoint),real(.05+(c%16)*.0125),real(.25+(c//4)%8*.0625),word(nseed),real(gain),real(gain*.25),word(0xabcd),real(0),real(0),real(60),real(0)])
 out+=word(count)+initial
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(0x2000000,0x10000);e.mem_write(address,code);e.mem_write(workspace,initial);e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 for block in range(3):
  data=[]
  for i in range(count*5):
   seed=(seed*1664525+1013904223)&0xffffffff;data.append(float((seed>>16)%1024-512)/512)
  data=b''.join(real(v) for v in data);out+=data
  e.mem_write(output_address,data[:count*4]);e.mem_write(symbols['auxabuf'],data[count*4:count*12]);e.mem_write(symbols['auxbbuf'],data[count*12:count*20]);e.reg_write(UC_X86_REG_EDI,output_address);e.reg_write(UC_X86_REG_ECX,count);e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)))
  e.emu_start(symbols['syOscRender'],address+len(code));out+=e.mem_read(workspace,60)+e.mem_read(output_address,count*4)
path=root/'tests/fixtures/synth-oscillator-render.bin';path.write_bytes(out)
print(f'{path}: {len(out)} bytes, 384 original oscillator render blocks; code SHA256 {hashlib.sha256(code).hexdigest()}')
