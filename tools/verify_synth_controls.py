#!/usr/bin/env python3
"""Original controller/program/note-off changes, recording voice/speech callbacks."""
import struct,hashlib,re
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import ROOT,voice_code
source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
text=source[source.index('ProcessNoteOff:'):source.index('ProcessNoteOn:')]+source[source.index('ProcessControlChange:'):source.index('ProcessPitchBend:')]
# The game enables FULLMIDI and RONAN. Select only their active branches.
lines=[];active=True;stack=[]
for line in text.splitlines():
 if line.strip().startswith('%ifdef'):
  stack.append(active);continue
 if line.strip()=='%else':active=False;continue
 if line.strip()=='%endif':active=stack.pop();continue
 if active:lines.append(line)
text='\n'.join(lines).replace('call\tsyV2Init','call syMarkReset').replace('call\t\tsyV2Init','call syMarkReset')
text=re.sub(r'call\s+syV2Init','call syMarkReset',text);text=re.sub(r'call\s+syV2NoteOff','call syMarkRelease',text)
text=text.replace('_ronanCBNoteOff@0','sySpeechRelease').replace('_ronanCBSetCtl@8','sySpeechControl')
text=re.sub(r'mov\s+ebp,\s*data\s*\+\s*SYN.voicesw','lea ebp,[data+SYN.voicesw]',text)
for name,target in [('Reset','resetmask'),('Release','releasemask')]:
 text+='\nsyMark'+name+':\npushad\nmov eax,ebp\nsub eax,data+SYN.voicesw\nxor edx,edx\nmov ecx,552\ndiv ecx\nmov ecx,eax\nmov eax,1\nshl eax,cl\nor ['+target+'],eax\npopad\nret\n'
text+='''
sySpeechRelease:
mov dword [speechkind],2
ret
sySpeechControl:
pushad
mov dword [speechkind],1
mov eax,[esp+36]
mov [speechindex],eax
mov eax,[esp+40]
mov [speechvalue],eax
popad
ret 8
'''
preamble=['.set data,0x2000000','.set SYN.samplerate,12','.set SYN.chanmap,16','.set SYN.chans,272','.set SYN.voicesw,8020','.set POLY,32','.set resetmask,0x2007000','.set releasemask,0x2007004','.set speechkind,0x2007008','.set speechindex,0x200700c','.set speechvalue,0x2007010']
code,symbols,fields=voice_code(extra_text=text,preamble=preamble)
word=lambda n:struct.pack('<I',n&0xffffffff)
address=0x1000000;data=0x2000000;packet=0x2010000;seed=1234;out=bytearray(word(512))
for c in range(512):
 def random():
  global seed
  seed=(seed*1664525+1013904223)&0xffffffff;return seed
 kind=[0,0,1,2][c%4];channel=(c//4)%16;rate=[44100,48000,96000,channel][c%4]
 info=bytearray(random()>>24 for _ in range(128))
 for i in range(16):info[i*8]&=127
 maps=[[-1,random()%16,random()%16-256][random()%3] for _ in range(32)]
 notes=[48+random()%25 for _ in range(32)];gates=[[0,1,256,257][random()%4] for _ in range(32)]
 first=[0,1,2,3,4,5,6,7,8,119,120,121,122,123,124,127,255][(c//4)%17] if kind==0 else (info[channel*8]+[0,1,128][(c//4)%3])&255 if kind==1 else notes[c%32]
 second=random()>>24;out+=word(kind)+word(channel)+word(rate)+word(first)+word(second)+info+b''.join(word(v) for v in maps+notes+gates)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(data,0x20000);e.mem_write(address,code);e.mem_write(data+12,word(rate));e.mem_write(data+16,b''.join(word(v) for v in maps));e.mem_write(data+272,bytes(info));e.mem_write(packet,bytes([first,second]))
 for i,(note,gate) in enumerate(zip(notes,gates)):e.mem_write(data+8020+i*552,word(note)+word(0)+word(gate))
 e.reg_write(UC_X86_REG_ECX,channel);e.reg_write(UC_X86_REG_ESI,packet);e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code)));e.emu_start(symbols[['ProcessControlChange','ProcessProgramChange','ProcessNoteOff'][kind]],address+len(code))
 out+=e.mem_read(data+12,132)+e.mem_read(data+272,128)+e.mem_read(data+0x7000,20)
path=ROOT/'tests/fixtures/synth-controls.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 512 controller/program/note-off states; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
