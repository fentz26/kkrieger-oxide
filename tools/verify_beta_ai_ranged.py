#!/usr/bin/env python3
"""Run released monster spawn through body/hit-cell construction, no stubs."""
import runpy,pathlib,struct,random,hashlib
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_ESI
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
# Fresh roomy arena for the environment and game structures.
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(b)+4095)&~4095);u.mem_write(base,b);u.mem_map(0x3000000,0x60000)
actor,op,data,env,game,cell,array,frame,memory=0x3001000,0x3003000,0x3004000,0x3010000,0x3020000,0x3006000,0x3007000,0x305f000,0x3000000
u.mem_map(0x5000000,4096)
def intercept(u,address,size,user):
 global damage,events
 if address in (0x801000,0x818047):
  esp=u.reg_read(UC_X86_REG_ESP);result=0
  if address==0x801000:
   result=0x3040000;u.mem_write(result,bytes(180))
  else:events.append(bytes(u.mem_read(0x3040000,180)))
  u.reg_write(UC_X86_REG_EAX,result);u.reg_write(UC_X86_REG_EIP,struct.unpack('<I',u.mem_read(esp,4))[0]);u.reg_write(UC_X86_REG_ESP,esp+4+(4 if address==0x818047 else 0));return
 if address==0x819cf4:
  esp=u.reg_read(UC_X86_REG_ESP);damage+=struct.unpack('<I',u.mem_read(esp+4,4))[0]
  u.reg_write(UC_X86_REG_EIP,struct.unpack('<I',u.mem_read(esp,4))[0]);u.reg_write(UC_X86_REG_ESP,esp+8);return
 if address not in (0x81c852,0x81b45a):return
 esp=u.reg_read(UC_X86_REG_ESP)
 value=(case//2)%2 if address==0x81c852 else 0
 u.reg_write(UC_X86_REG_EAX,value);u.reg_write(UC_X86_REG_EIP,struct.unpack('<I',u.mem_read(esp,4))[0]);u.reg_write(UC_X86_REG_ESP,esp+4+(16 if address==0x81c852 else 32))
u.hook_add(UC_HOOK_CODE,intercept)
rng=random.Random(0x200432);out=bytearray(pack(32))
for case in range(32):
 words=[0]*26
 for i,v in [(6,1.),(7,50.),(8,50.),(13,1.),(15,2.85),(16,0.5),(18,50.),(19,1.),(20,1.25),(21,1.),(22,1.),(24,0.5)]:words[i]=struct.unpack('<I',struct.pack('<f',v))[0]
 words[10]=20;words[12]=0;words[14]=5
 if case%3:
  for i in range(3):words[i]=struct.unpack('<I',struct.pack('<f',rng.uniform(-0.5,0.5)))[0]
 words[13]=struct.unpack('<I',struct.pack('<f',0.01))[0];words[15]=words[13]
 parent=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,5.,0.,0.,1.]
 present=case%2
 u.mem_write(actor,bytes(0x7e0));u.mem_write(op,bytes(0xa0));u.mem_write(op+0x94,pack(data));u.mem_write(data,pack(*words));u.mem_write(env+0x94f0,struct.pack('<16f',*parent));u.mem_write(env+0x1358,pack(game))
 planes=[[1.,0.,0.,20.],[-1.,0.,0.,20.],[0.,1.,0.,20.],[0.,-1.,0.,20.],[0.,0.,1.,20.],[0.,0.,-1.,20.]]
 u.mem_write(cell+32,struct.pack('<24f',*[v for p in planes for v in p]));u.mem_write(cell+0xf0,pack(6));u.mem_write(array,pack(cell));u.mem_write(game+4,pack(array,present))
 u.mem_write(frame+8,pack(op,env));u.mem_write(frame-4,pack(memory));u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_EBX,actor);u.reg_write(UC_X86_REG_ESI,op);u.reg_write(UC_X86_REG_ESP,frame-0xb4);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x81d81e,0x81d9b4,count=100000)
 u.mem_write(game+0x14948,struct.pack('<4f',0.,0.,0.,1.));u.mem_write(game+0x15394,struct.pack('<f',0.01));u.mem_write(game+0x153a0,struct.pack('<2f',0.2,1.))
 u.mem_write(frame,pack(0x5000000,actor,0x3050000));u.reg_write(UC_X86_REG_ESP,frame);u.reg_write(UC_X86_REG_ECX,game)
 damage=0;events=[];u.mem_write(actor+0x1e0,pack(0x12345678))
 for tick in range(case%16+1):
  u.mem_write(frame,pack(0x5000000,actor,0x3050000));u.reg_write(UC_X86_REG_ESP,frame);u.reg_write(UC_X86_REG_ECX,game)
  u.emu_start(0x81d33e,0x5000000,count=100000)
 out.extend(pack(actor,present,*words));out.extend(struct.pack('<16f',*parent));out.extend(u.mem_read(actor+0x1f4,640));out.extend(u.mem_read(actor+0x78,180));out.extend(u.mem_read(actor+0x1e8,12));out.extend(pack(len(events)));out.extend(b''.join(events))
p=root/'tests/fixtures/beta-ai-ranged.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
