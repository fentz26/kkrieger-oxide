#!/usr/bin/env python3
"""Execute original x86 UpdateSampleDelta instructions in Unicorn."""
import pathlib,re,struct,subprocess,tempfile,hashlib
from unicorn import Uc, UcError, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_EFLAGS
root=pathlib.Path(__file__).resolve().parents[1]
source=(root/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2.cpp').read_text(encoding='cp1252')
a=source.index('void UpdateSampleDelta(');a=source.index('__asm {',a)+len('__asm {');b=source.index('}',a);body=source[a:b].strip()
code_address=0x1000000;data_address=0x2000000
names=['nexttime','time','usecs','td2','smplrem','smpldelta']
assembly=body
for i,name in enumerate(names):assembly=assembly.replace('['+name+']',f'[{data_address+i*4}]')
with tempfile.TemporaryDirectory() as tmp:
 tmp=pathlib.Path(tmp);asm=tmp/'delta.s';obj=tmp/'delta.o'
 asm.write_text('.intel_syntax noprefix\n.text\n'+assembly+'\n')
 subprocess.run(['clang','-target','i386-unknown-linux-gnu','-c',str(asm),'-o',str(obj)],check=True)
 elf=obj.read_bytes();shoff=struct.unpack_from('<I',elf,32)[0];size,count,strings=struct.unpack_from('<HHH',elf,46)
 headers=[struct.unpack_from('<10I',elf,shoff+i*size) for i in range(count)];h=headers[strings];names_data=elf[h[4]:h[4]+h[5]]
 text=next(h for h in headers if names_data[h[0]:].split(b'\0')[0]==b'.text');code=elf[text[4]:text[4]+text[5]]
word=lambda n:struct.pack('<I',n&0xffffffff)
out=bytearray(word(512));seed=0x1234
for c in range(512):
 def random():
  global seed
  seed=(seed*1664525+1013904223)&0xffffffff
  return seed
 time=random();next_time=(time+random()%0x1000000)&0xffffffff;usecs=random();division=[1,10,480000,960000,9600000][c%5];remainder=random()
 if c%16==0:division=0
 if c%17==0:next_time=time
 if c%7==0:remainder=0xffffffff
 inputs=[next_time,time,usecs,division,remainder]
 for value in inputs:out+=word(value)
 emulator=Uc(UC_ARCH_X86,UC_MODE_32);emulator.mem_map(code_address,4096);emulator.mem_map(data_address,4096);emulator.mem_write(code_address,code)
 emulator.mem_write(data_address,b''.join(word(v) for v in [next_time,time,usecs,division,data_address+0x100,data_address+0x104]));emulator.mem_write(data_address+0x100,word(remainder)+word(0xdeadbeef));emulator.reg_write(UC_X86_REG_EFLAGS,2)
 try:emulator.emu_start(code_address,code_address+len(code));success=True
 except UcError:success=False
 out+=word(success)+emulator.mem_read(data_address+0x100,8)
path=root/'tests/fixtures/v2m-sample-delta.bin';path.write_bytes(out)
print(f'{path}: {len(out)} bytes; 512 x86 sample-delay calculations; instruction SHA256 {hashlib.sha256(code).hexdigest()}')
