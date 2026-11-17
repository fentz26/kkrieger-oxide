#!/usr/bin/env python3
"""Run original reverb in a native x86 executable, including x87 underflow flags."""
import re,struct,subprocess,tempfile
from synth_asm import ROOT,voice_code
source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
_,_,fields=voice_code(sections=[('lencl0 \tequ','global _V2CHAN_')])
text=source[source.index('syRvDefs  dd'):source.index('global _V2CHAN_')]
a=source.index('pow:');text+='\n'+source[a:source.index(';#####################################################################################',a)]
lines=['.intel_syntax noprefix','.text']+[f'.set {k},{v}' for k,v in fields.items()];scope=''
registers=['rax','rcx','rdx','rbx','rbp','rsi','rdi']
for line in text.splitlines():
 line=line.split(';')[0].strip()
 if not line or line.startswith(('%','V2Perf')):continue
 if line=='pushad':lines += ['push '+r for r in registers];continue
 if line=='popad':lines += ['pop '+r for r in reversed(registers)];continue
 if re.fullmatch(r'sy\w+:?',line):line=line.rstrip(':')+':'
 if line.endswith(':') and not line.startswith('.'):scope=line[:-1]
 line=re.sub(r'(?<![\w.])\.(\w+)',lambda m:scope+'.'+m[1],line)
 line=re.sub(r'\b(dword|qword|word|byte)\s+\[',r'\1 ptr [',line)
 line=re.sub(r'\bbyte\s+(-?\d+)',r'\1',line)
 line=re.sub(r'\bnear\s+','',line)
 line=re.sub(r'\bst([0-7])\b',r'st(\1)',line)
 line=re.sub(r'fxch\s+st\(0\),\s*','fxch ',line)
 # Pointer registers widen to the host ABI; all counters and DSP remain x86.
 line=re.sub(r'(?<=\[)([^\]]+)',lambda m:re.sub(r'\be(bp|si|di|dx|cx)\b',lambda n:'r'+n[1],m[0]),line)
 line=re.sub(r'^lea\s+e(bp|si|di),',r'lea r\1,',line)
 line=line.replace('mov    edi, ebp','mov rdi, rbp')
 line=line.replace('[syRvDefs+4*rcx]','[r11+4*rcx]')
 if '[r11+' in line:lines.append('lea r11,[rip+syRvDefs]')
 line=re.sub(r'\[(aux1buf|fc64|fci128|SRfclinfreq|dcoffset)\]',lambda m:'[rip+'+('_aux1buf' if m[1]=='aux1buf' else m[1])+']',line)
 if re.search(r'\bdd\s',line):
  label,data=re.split(r'\bdd\s+',line,maxsplit=1);line=(label.strip()+': ' if label.strip() else '')+'.long '+','.join(str(int.from_bytes(struct.pack('<f',float(v)),'little')) for v in data.split(','))
 elif re.fullmatch(r'\w+\.\w+',line):line+=':'
 lines.append(line)
lines += ['.globl _native_process','_native_process:','push rbp','mov rbp,rdi','mov rdi,rsi','mov ecx,edx','call syReverbProcess','pop rbp','ret','.data','.globl _aux1buf','_aux1buf: .zero 1120']
for name,value in [('fc64',64.),('fci128',1/128),('SRfclinfreq',1.),('dcoffset',2**-18)]:lines.append(f'{name}: .long '+str(int.from_bytes(struct.pack('<f',value),'little')))
cpp=r'''
#include <cstdio>
#include <vector>
#include <cstring>
extern "C" void native_process(void*,void*,int);
extern "C" float aux1buf[];
void word(unsigned n){fwrite(&n,4,1,stdout);}
int main(){unsigned short cw=0x7f;asm volatile("fldcw %0"::"m"(cw));word(64);word(SIZE/4);unsigned seeds[]={1,2,3,7,0x7fffff,0x800000,0x800001,0xffffff,0x1000000};for(unsigned c=0;c<64;c++){std::vector<unsigned>w(SIZE/4);float ga[]={.5f,.1f,.75f,1.f},gb[]={.5f,.6f,.25f,0.f};float coeffs[]={0,0,0,0,ga[c%4],gb[(c/4)%4],0,0,(c%3)*.5f};memcpy(w.data(),coeffs,36);fwrite(coeffs,4,9,stdout);unsigned offsets[]={AL0,AL1,AR0,AR1};for(int j=0;j<4;j++){unsigned bits=seeds[(c+j)%9]|((c&8)?0x80000000:0);w[offsets[j]/4]=bits;word(bits);}unsigned n=1+c%7;word(n);std::vector<float>out(2*n);memset(aux1buf,0,4*n);native_process(w.data(),out.data(),n);fwrite(out.data(),4,2*n,stdout);fwrite(w.data(),4,31,stdout);unsigned count=0;for(unsigned i=31;i<w.size();i++)if(w[i])count++;word(count);for(unsigned i=31;i<w.size();i++)if(w[i]){word(i-31);word(w[i]);}}}
'''
for name,value in {'SIZE':fields['syWReverb.size'],'AL0':fields['syWReverb.lineal0'],'AL1':fields['syWReverb.lineal1'],'AR0':fields['syWReverb.linear0'],'AR1':fields['syWReverb.linear1']}.items():cpp=cpp.replace(name,str(value))
with tempfile.TemporaryDirectory() as tmp:
 from pathlib import Path
 tmp=Path(tmp);asm=tmp/'reverb.s';driver=tmp/'driver.cpp';exe=tmp/'native';asm.write_text('\n'.join(lines));driver.write_text(cpp)
 subprocess.run(['clang++','-arch','x86_64','-std=c++17','-O2',str(driver),str(asm),'-o',str(exe)],check=True)
 result=subprocess.check_output(['arch','-x86_64',str(exe)])
path=ROOT/'tests/fixtures/synth-reverb-underflow.bin';path.write_bytes(result);print(f'{len(result)} bytes; 64 native x87 reverb underflow cases')
