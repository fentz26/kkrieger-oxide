#!/usr/bin/env python3
"""Original distortion Init/Set/mono/stereo processing and nested filters."""
import pathlib,re,struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_ECX,UC_X86_REG_EBP,UC_X86_REG_ESP,UC_X86_REG_FPCW
from x86_asm import assemble
root=pathlib.Path(__file__).resolve().parents[1]
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('syDistInit:');b=source.index('; V2 - Voice',a);text=source[a:b]
a=source.index('syFltInit:');b=source.index('; Low Frequency Oscillator',a);text+='\n'+source[a:b]
a=source.index('fastatan:');b=source.index('global fastsinrc',a);text+='\n'+source[a:b]
a=source.index('calcfreq:');b=source.index('pow2:',a);text+='\n'+source[a:b]
a=source.index('pow2:');b=source.index('\npow:',a);text+='\n'+source[a:b]
fields={
 'syVDist':dict(zip(['mode','ingain','param1','param2'],range(0,16,4))),
 'syWDist':dict(zip(['mode','gain1','gain2','offs','crush1','crush2','crxor','dcount','dfreq','dvall','dvalr','dlp1c','dlp1bl','dlp1br','dlp2c','dlp2bl','dlp2br'],range(0,68,4)))|{'fw1':68,'fw2':92},
 'syVFlt':dict(zip(['mode','cutoff','reso'],range(0,12,4))),
 'syWFlt':dict(zip(['mode','cfreq','res','l','b','step'],range(0,24,4))),
}
for prefix,mapping in fields.items():
 for name,offset in mapping.items():text=re.sub(r'\b'+prefix+r'\.'+name+r'\b',str(offset),text)
lines=[];scope=''
for line in text.splitlines():
 line=line.split(';')[0].strip()
 if not line or line.startswith(('section','global','V2Perf','%')):continue
 if line.endswith(':') and not line.startswith('.'):scope=line[:-1]
 line=re.sub(r'(?<![\w.])\.(\w+)',lambda m:scope+'.'+m[1],line)
 line=re.sub(r'\b([0-9][0-9a-fA-F]*)h\b',r'0x\1',line)
 line=re.sub(r'\b(dword|qword|word|byte)\s+\[',r'\1 ptr [',line)
 line=re.sub(r'\bbyte\s+(-?(?:0x[0-9a-fA-F]+|[0-9]+))',r'\1',line)
 line=re.sub(r'\bst([0-7])\b',r'st(\1)',line)
 if re.search(r'\bdd\s',line):
  label,data=re.split(r'\bdd\s+',line,maxsplit=1);line=(label.strip()+': ' if label.strip() else '')+'.long '+data.rstrip(',')
 elif re.fullmatch(r'sy\w+\.\w+',line):line+=':'
 lines.append(line)
for name in ['fc32','fci16','fci128','fc64','fc256','fc32768','fc32bit','fci127','fci32768','fc10','SRfclinfreq','dcoffset','fcatanx1','fcatanx3','fcatanxm0','fcatanxm4','fcatanxm2','fcatanadd']:
 kind,value=re.search(r'^'+name+r'\s+(dd|dq)\s+([^;\r\n]+)',source,re.M).groups();double=kind=='dq';bits=[int.from_bytes(struct.pack('<d' if double else '<f',float(v.strip())),'little') for v in value.split(',')];lines.append(f'{name}: '+('.quad ' if double else '.long ')+','.join(map(str,bits)))
lines.append('temp: .zero 64')
address=0x1000000;workspace=0x2000100;parameters=0x2000300;input_address=0x2001000;output_address=0x2003000
code,symbols=assemble('\n'.join(lines),address)
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(128)+word(3));seed=1234
for c in range(128):
 stereo=(c//16)%2==1;alias=c%3==0;count=[1,2,7,128,280][c%5];length=count*(2 if stereo else 1)
 values=[float(c%16),float(c*7%129),float(c*11%129),float(c*13%129)]
 out+=b''.join(real(v) for v in values)+word(stereo)+word(alias)+word(count)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(0x2000000,0x10000);e.mem_write(address,code);e.mem_write(parameters,b''.join(real(v) for v in values));e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)));e.emu_start(symbols[name],address+len(code))
 call('syDistInit');e.reg_write(UC_X86_REG_ESI,parameters);call('syDistSet');out+=e.mem_read(workspace,116)
 for block in range(3):
  values=[]
  for i in range(length):
   seed=(seed*1664525+1013904223)&0xffffffff;values.append(float((seed>>16)%2048-1024)/512)
  data=b''.join(real(v) for v in values);out+=data;e.mem_write(input_address,data);e.mem_write(output_address,real(-777)*length)
  e.reg_write(UC_X86_REG_ESI,input_address);e.reg_write(UC_X86_REG_EDI,input_address if alias else output_address);e.reg_write(UC_X86_REG_ECX,count);call('syDistRenderStereo' if stereo else 'syDistRenderMono')
  out+=e.mem_read(workspace,116)+e.mem_read(input_address if alias else output_address,length*4)
path=root/'tests/fixtures/synth-distortion.bin';path.write_bytes(out)
print(f'{path}: {len(out)} bytes, 128 distortion setups and 384 mono/stereo render blocks; code SHA256 {hashlib.sha256(code).hexdigest()}')
