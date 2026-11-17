#!/usr/bin/env python3
"""Original x87 sine helper executed in a native x86-64 process (Rosetta on ARM).

Unicorn's FPREM loses -0 for exact negative multiples, so this helper needs an
independent instruction runner. Only addressing and the C-call wrapper change.
"""
import pathlib,re,struct,subprocess,tempfile,math,sys
root=pathlib.Path(__file__).resolve().parents[1]
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('fastsinrc:');b=source.index('global calcNewSampleRate',a);text=source[a:b]
lines=[]
for line in text.splitlines():
 line=line.split(';')[0].strip()
 if not line:continue
 line=re.sub(r'\b(dword|qword)\s+\[',r'\1 ptr [rip+',line)
 line=re.sub(r'\bst([0-7])\b',r'st(\1)',line);lines.append(line)
name='_sine_check' if sys.platform=='darwin' else 'sine_check'
wrapper=f'''.intel_syntax noprefix
.text
.global {name}
{name}:
sub rsp, 24
movss dword ptr [rsp], xmm0
fninit
fldcw word ptr [rip+control]
fld dword ptr [rsp]
call fastsinrc
fstp dword ptr [rsp]
movss xmm0, dword ptr [rsp]
add rsp, 24
ret
'''
for symbol in ['fcpi_2','fc1p5pi','fc2pi','fcsinx3','fcsinx5','fcsinx7']:
 kind,value=re.search(r'^'+symbol+r'\s+(dd|dq)\s+([^;\r\n]+)',source,re.M).groups();double=kind=='dq';bits=int.from_bytes(struct.pack('<d' if double else '<f',float(value.strip())),'little');lines.append(f'{symbol}: '+('.quad ' if double else '.long ')+str(bits))
lines.append('control: .short 0x7f')
driver=r'''
#include <cstdio>
extern "C" float sine_check(float);
int main(int argc,char**argv){FILE*f=fopen(argv[1],"rb");unsigned count=2048;fwrite(&count,4,1,stdout);for(unsigned i=0;i<count;i++){float input;fread(&input,4,1,f);float output=sine_check(input);fwrite(&input,4,1,stdout);fwrite(&output,4,1,stdout);}fclose(f);}
'''
inputs=bytearray()
for i in range(2048):
 value=[0.,-0.,math.pi/2,math.pi,math.pi*1.5,math.pi*2,-math.pi/2,-math.pi,-math.pi*2][i] if i<9 else (i-1024)*.015625
 inputs+=struct.pack('<f',value)
with tempfile.TemporaryDirectory() as tmp:
 tmp=pathlib.Path(tmp);asm=tmp/'sine.s';cpp=tmp/'main.cpp';data=tmp/'input.bin';exe=tmp/'sine'
 asm.write_text(wrapper+'\n'.join(lines)+'\n');cpp.write_text(driver);data.write_bytes(inputs)
 subprocess.run(['clang++',*(['-arch','x86_64'] if sys.platform=='darwin' else []),'-std=c++17','-O2',str(cpp),str(asm),'-o',str(exe)],check=True)
 result=subprocess.check_output([str(exe),str(data)])
path=root/'tests/fixtures/synth-fast-sine.bin';path.write_bytes(result);print(f'{path}: {len(result)} bytes, 2048 native x87 sine/range-reduction cases')
