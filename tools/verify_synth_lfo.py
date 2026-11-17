#!/usr/bin/env python3
"""Original LFO Set/KeyOn/Tick and fast sine under x87 emulation."""
import pathlib,re,struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBP,UC_X86_REG_ESP,UC_X86_REG_FPCW
from x86_asm import assemble
root=pathlib.Path(__file__).resolve().parents[1]
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('\nsyLFOSet\n');b=source.index('; INDUSTRIAL',a);text=source[a:b]
a=source.index('fastsinrc:');b=source.index('global calcNewSampleRate',a);text+='\n'+source[a:b]
a=source.index('calcfreq:');b=source.index('pow2:',a);text+='\n'+source[a:b]
for i,name in enumerate(['mode','sync','egmode','rate','phase','pol','amp']):text=text.replace('syVLFO.'+name,str(i*4))
for i,name in enumerate(['out','mode','fs','feg','freq','cntr','cph','gain','dc','nseed','last']):text=text.replace('syWLFO.'+name,str(i*4))
text=text.replace('icnoisemul','196314165').replace('icnoiseadd','907633515')
lines=[];scope=''
for line in text.splitlines():
 line=line.split(';')[0].strip()
 if not line:continue
 if re.match(r'^syLFO\w+$',line):scope=line;line+=':'
 elif line.endswith(':') and not line.startswith('.'):scope=line[:-1]
 line=re.sub(r'(?<![\w.])\.(\w+)',lambda m:scope+'.'+m[1],line)
 line=re.sub(r'\b([0-9][0-9a-fA-F]*)h\b',r'0x\1',line)
 line=re.sub(r'\b(dword|qword|byte)\s+\[',r'\1 ptr [',line)
 line=re.sub(r'\bst([0-7])\b',r'st(\1)',line)
 if re.search(r'\bdd\s',line):
  label,data=re.split(r'\bdd\s+',line,maxsplit=1);line=(label.strip()+': ' if label.strip() else '')+'.long '+data.rstrip(',')
 elif re.fullmatch(r'[\w.]+',line) and line.startswith('syLFO'):line+=':'
 lines.append(line)
for name in ['fci128','fc32bit','fci2','fc10','fcpi_2','fc1p5pi','fc2pi','fcsinx3','fcsinx5','fcsinx7']:
 kind,value=re.search(r'^'+name+r'\s+(dd|dq)\s+([^;\r\n]+)',source,re.M).groups();double=kind=='dq';bits=int.from_bytes(struct.pack('<d' if double else '<f',float(value.strip())),'little');lines.append(f'{name}: '+('.quad ' if double else '.long ')+str(bits))
lines.append('temp: .zero 64')
address=0x1000000;workspace=0x2000100;parameters=0x2000200
code,symbols=assemble('\n'.join(lines),address)
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(128)+word(64))
for c in range(128):
 values=[float(c%8),float((c//8)%2),float((c//16)%2),float(c*7%128)+(c%2)*.5,float(c*17%129),float(c%4),float(c*13%128)+.25]
 if c%16==0:values[3]=[-1.,0.,64.,127.,128.,255.][(c//16)%6]
 counter=[0,0xfffffff0,0x7fffffff,0x80000000,0x40000000,0xc0000000,1,0xaaaaaaaa][c%8];seed=(c*196314165+1234)&0xffffffff;last=0
 data=b''.join(real(v) for v in values);out+=data+word(counter)+word(seed)+word(last)
 initial=real(16)+word(0)*4+word(counter)+word(0)+real(17)*2+word(seed)+word(last)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,4096);e.mem_map(0x2000000,4096);e.mem_write(address,code);e.mem_write(parameters,data);e.mem_write(workspace,initial);e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_ESI,parameters);e.reg_write(UC_X86_REG_FPCW,0x7f)
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x2000800);e.mem_write(0x2000800,word(address+len(code)));e.emu_start(symbols[name],address+len(code))
 call('syLFOSet');out+=e.mem_read(workspace,44)
 for k in range(64):
  key=k%23==0;out+=word(key)
  if key:call('syLFOKeyOn')
  call('syLFOTick');out+=e.mem_read(workspace,44)
path=root/'tests/fixtures/synth-lfo.bin';path.write_bytes(out)
print(f'{path}: {len(out)} bytes, 128 LFO conversions and 8192 x87 LFO ticks; code SHA256 {hashlib.sha256(code).hexdigest()}')
