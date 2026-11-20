#!/usr/bin/env python3
"""Run released text placement with deterministic GDI extents; capture calls/UVs.

Only color conversion, bitmap ownership validation and GDI/memory services are
replaced. Layout/range parsing and FontWidth/FontPrint execute original code.
No native glyph rasterization or final bitmap blend is claimed.
"""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
root=pathlib.Path(__file__).resolve().parent.parent
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(0x7c0000,(len(image)+4095)&~4095);u.mem_write(0x7c0000,image)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,4096)
env,bm,text,font,stack=0x3000000,0x3010000,0x3011000,0x3012000,0x303f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*[x&0xffffffff for x in v])
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
floatword=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
def string(p,n=None):
 if n is not None:return bytes(u.mem_read(p,n))
 out=bytearray()
 while (b:=u.mem_read(p,1)[0]):out.append(b);p+=1
 return bytes(out)
def ret(pop,value=0):
 esp=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EAX,value&0xffffffff);u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+4+pop)
trace=[];begin=[];height=0
# IAT entries used by the released FontWidth and FontPrint.
u.mem_write(0x81f00c,pack(0x5000100));u.mem_write(0x81f008,pack(0x5000200))
def hook(u,address,size,user):
 if address==0x80cb66:u.emu_stop();return
 if address in [0x80a63e,0x80a608]:
  u.reg_write(UC_X86_REG_EDX,0);ret(0);return
 if address==0x801f53:
  esp=u.reg_read(UC_X86_REG_ESP);args=[read(esp+4+i*4) for i in range(6)]
  assert args[2]==font;begin.extend(args[:2]+args[3:]);ret(24,height);return
 if address==0x5000100:
  esp=u.reg_read(UC_X86_REG_ESP);dc,p,n,out=[read(esp+4+i*4) for i in range(4)]
  value=sum((b%13)+1 for b in string(p,n));u.mem_write(out,pack(value,height));ret(16,1);return
 if address==0x5000200:
  esp=u.reg_read(UC_X86_REG_ESP);args=[read(esp+4+i*4) for i in range(8)]
  assert args[3:5]==[0,0] and args[7]==0
  data=string(args[5],args[6]);trace.append((args[1],args[2],data));ret(32,1)
u.hook_add(UC_HOOK_CODE,hook)
rng=random.Random(0x62666e74)
texts=[b'a-c!',b'a-c',b'--!',b'a-zA-Z0-9.,-:;><! ',b'',b'   ',b'Z-A!',b'abc']
out=bytearray(pack(96))
for case in range(96):
 w=[16,32,64,128][case%4];h=[8,16,32][case%3];height=9+case%19
 params=[floatword(rng.uniform(-.2,.4)),floatword(rng.uniform(-.2,.4)),floatword(rng.uniform(.02,.4)),floatword(rng.uniform(0,.1)),0xffffffff,16 if case%5 else 1,(case%5)-1,floatword([0.,.003,-.002][case%3])]
 if case%12==0:params[7]=163
 content=texts[case%len(texts)];u.mem_write(env,bytes(0x10000));u.mem_write(bm,bytes(64));u.mem_write(bm+0x10,pack(w,h));u.mem_write(text,content+b'\0');u.mem_write(font,b'Arial\0');u.mem_write(0x83f868,pack(0x3014000));u.mem_write(0x83f874,pack(0x1234))
 u.mem_write(stack,pack(0x5000000,env,bm,*params,text,font));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_FPCW,0x143f)
 trace=[];begin=[];u.emu_start(0x80c951,0x5000000,count=100000)
 assert u.reg_read(UC_X86_REG_EIP)==0x80cb66, "initializer did not reach bitmap readback"
 assert len(begin)==5
 out.extend(pack(w,h,height,*params,len(content)));out.extend(content);out.extend(pack(*begin,len(trace)))
 for x,y,data in trace:out.extend(pack(x,y,len(data)));out.extend(data)
 out.extend(u.mem_read(env+0x14f0+4096,4096))
p=root/'tests/fixtures/beta-font-layout.bin';p.write_bytes(out);print(len(out),hashlib.sha256(out).hexdigest())
