#!/usr/bin/env python3
"""Original MIDI packet parser with dispatch-recording handler adapters."""
import struct,hashlib
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import *
from synth_asm import ROOT,voice_code
source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
a=source.index('_synthProcessMIDI@4:');a=source.index('pushad',a);b=source.index('; esi: ptr auf midistream',a)
text='syMidi:\n'+source[a:b].replace('popad','mov [consumed],esi\npopad')
handlers=['NoteOff','NoteOn','PolyPressure','ControlChange','ProgramChange','ChannelPressure','PitchBend','RealTime'];lengths=[2,2,2,2,1,1,2,0]
text+='\nspMTab dd '+','.join('syRecord'+name for name in handlers)+'\n'
for name,n in zip(handlers,lengths):
 text+='syRecord'+name+':\nmov ecx,'+str(n)+'\ncall syRecord\n'
 if n:text+='add esi,'+str(n)+'\n'
 else:text+='cmp dword [data+SYN.mrstat],255\njne .done\nmov dword [data+SYN.mrstat],0\n.done\n'
 text+='ret\n'
text+='''syRecord:
pushad
mov edi,[tracecount]
shl edi,4
lea edi,[tracebuffer+edi]
mov eax,[data+SYN.mrstat]
mov [edi],eax
xor eax,eax
mov [edi+4],eax
mov [edi+8],eax
mov [edi+12],ecx
or ecx,ecx
jz .done
mov al,[esi]
mov [edi+4],eax
cmp ecx,2
jne .done
mov al,[esi+1]
mov [edi+8],eax
.done
inc dword [tracecount]
popad
ret
'''
preamble=['.set data,0x2000000','.set SYN.mrstat,4','.set tracecount,0x2000008','.set consumed,0x200000c','.set tracebuffer,0x2000100','oldfpcw: .zero 4']
code,symbols,_=voice_code(extra_text=text,preamble=preamble)
word=lambda n:struct.pack('<I',n&0xffffffff)
address=0x1000000;data=0x2000000;packet_address=0x2010000;seed=1234;out=bytearray(word(256)+word(4))
for c in range(256):
 status=[0,0x80,0x91,0xb2,0xc3,0xd4,0xe5][c%7];out+=word(status)
 e=Uc(UC_ARCH_X86,UC_MODE_32);e.mem_map(address,((len(code)+4095)//4096)*4096);e.mem_map(data,0x20000);e.mem_write(address,code);e.mem_write(data+4,word(status));e.reg_write(UC_X86_REG_FPCW,0x7f)
 def random():
  global seed
  seed=(seed*1664525+1013904223)&0xffffffff;return seed
 for block in range(4):
  packet=bytearray()
  if block==3 and c%4==0:packet+=bytes([0xff,0xfd])
  elif block==3 and c%4==1:packet+=bytes([0xf0+c%13,0xfd])
  elif block==0 and c%17==0 and status==0:packet+=bytes([1,2,0xfd])
  else:
   for i in range(1+c%20):
    if i==0 or random()%3==0:status=0x80+(random()%7)*16+random()%16;packet.append(status)
    n=lengths[(status>>4)-8]
    packet.extend((random()>>24)&127 for _ in range(n))
   packet.append(0xfd)
  packet+=bytes([11,22,33]);out+=word(len(packet))+packet
  e.mem_write(packet_address,bytes(packet));e.mem_write(data+8,word(0));e.reg_write(UC_X86_REG_ESP,0x200f000);e.mem_write(0x200f000,word(address+len(code))+word(packet_address));e.emu_start(symbols['syMidi'],address+len(code))
  out+=e.mem_read(data+4,8)+word(struct.unpack('<I',e.mem_read(data+12,4))[0]-packet_address)
  n=struct.unpack('<I',e.mem_read(data+8,4))[0];out+=e.mem_read(data+256,n*16)
path=ROOT/'tests/fixtures/synth-midi.bin';path.write_bytes(out)
print(f'{len(out)} bytes; 1024 original MIDI packets; assembly SHA256 {hashlib.sha256(code).hexdigest()}')
