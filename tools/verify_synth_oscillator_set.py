#!/usr/bin/env python3
"""Original sample-rate setup and oscillator pitch/waveform coefficients."""
import pathlib,re,struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_ESI,UC_X86_REG_EBP,UC_X86_REG_ESP,UC_X86_REG_FPCW
from x86_asm import assemble
root=pathlib.Path(__file__).resolve().parents[1]
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('calcNewSampleRate:');b=source.index('calcfreq2:',a);text=source[a:b]
a=source.index('syOscChgPitch:');b=source.index('; edi: dest buf',a);text+='\n'+source[a:b]
a=source.index('calcfreq:');b=source.index('pow2:',a);text+='\n'+source[a:b]
a=source.index('pow2:');b=source.index('\npow:',a);text+='\n'+source[a:b]
for prefix,fields in [('syVOsc',['mode','ring','pitch','detune','color','gain']),('syWOsc',['mode','ring','cnt','freq','brpt','nffrq','nfres','nseed','gain','gain4','tmp','nfl','nfb','note','pitch'])]:
 for i,name in enumerate(fields):text=re.sub(r'\b'+prefix+r'\.'+name+r'\b',str(i*4),text)
lines=[]
for line in text.splitlines():
 line=line.split(';')[0].strip()
 if not line:continue
 line=re.sub(r'\b(dword|byte)\s+\[',r'\1 ptr [',line)
 line=re.sub(r'\bst([0-7])\b',r'st(\1)',line);lines.append(line)
for name in ['fcoscbase','fc32bit','fcsrbase','fc2pi','fcboostfreq','fcframebase','fci2','fc64','fci128','fcOscPitchOffs','fci12','fci4','fc10','SRfcobasefrq','SRfclinfreq','SRfcBoostSin','SRfcBoostCos','SRcFrameSize']:
 value=re.search(r'^'+name+r'\s+dd\s+([^;\r\n]+)',source,re.M).group(1).strip();bits=int.from_bytes(struct.pack('<f',float(value)),'little') if name!='SRcFrameSize' else int(value);lines.append(f'{name}: .long {bits}')
lines.append('temp: .zero 64')
address=0x1000000;workspace=0x2000100;parameters=0x2000200
code,symbols=assemble('\n'.join(lines),address)
word=lambda n:struct.pack('<I',n&0xffffffff)
real=lambda n:struct.pack('<f',n)
out=bytearray(word(256))
for c in range(256):
 sample_rate=[8000,11025,16000,22050,32000,44100,48000,88200,96000][c%9];note=float(c*13%128)+(c%4)*.125;counter=c*1234567;seed=(c*196314165+1234)&0xffffffff
 values=[float(c%8),float(c%2),float(c*7%129),float(c*5%129),float(c*3%129),float(c*11%129)]
 out+=word(sample_rate)+real(note)+word(counter)+word(seed)+b''.join(real(v) for v in values)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,4096);e.mem_map(0x2000000,0x10000);e.mem_write(address,code);e.mem_write(parameters,b''.join(real(v) for v in values));e.reg_write(UC_X86_REG_EBP,workspace);e.reg_write(UC_X86_REG_FPCW,0x7f)
 def call(name):
  e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)));e.emu_start(symbols[name],address+len(code))
 e.reg_write(UC_X86_REG_EAX,sample_rate);call('calcNewSampleRate')
 for name in ['SRfcobasefrq','SRfclinfreq','SRfcBoostSin','SRfcBoostCos','SRcFrameSize']:out+=e.mem_read(symbols[name],4)
 e.mem_write(workspace+8,word(counter));e.mem_write(workspace+28,word(seed));e.mem_write(workspace+52,real(note));e.reg_write(UC_X86_REG_ESI,parameters);call('syOscSet');out+=e.mem_read(workspace,60)
 for n in [note+1.25,note-2.5]:
  out+=real(n);e.mem_write(workspace+52,real(n));call('syOscChgPitch');out+=e.mem_read(workspace,60)
path=root/'tests/fixtures/synth-oscillator-set.bin';path.write_bytes(out)
print(f'{path}: {len(out)} bytes, 256 original sample-rate/oscillator setups and 512 pitch changes; code SHA256 {hashlib.sha256(code).hexdigest()}')
