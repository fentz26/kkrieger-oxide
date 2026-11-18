#!/usr/bin/env python3
"""Execute AI activation gate with controlled visibility, actual activation lookup."""
import runpy,pathlib,struct,hashlib
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EAX,UC_X86_REG_EIP
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_actor_spawn.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(b)+4095)&~4095);u.mem_write(base,b);u.mem_map(0x3000000,0x60000)
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
casts=0

def hook(u,address,size,user):
 global casts
 if address!=0x81c852:return
 casts+=1;assert u.reg_read(UC_X86_REG_ECX)==game
 esp=u.reg_read(UC_X86_REG_ESP);assert read(esp+16)==0
 u.reg_write(UC_X86_REG_EAX,occluded);u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+20)
u.hook_add(UC_HOOK_CODE,hook)
planes=[[1.,0.,0.,20.],[-1.,0.,0.,20.],[0.,1.,0.,20.],[0.,-1.,0.,20.],[0.,0.,1.,20.],[0.,0.,-1.,20.]]
u.mem_write(cell+32,struct.pack('<24f',*[v for p in planes for v in p]));u.mem_write(cell+0xf0,pack(6));u.mem_write(array,pack(cell))
out=bytearray(pack(144))
for case in range(144):
 active=case%2;occluded=(case//2)%2;present=(case//4)%2
 distance=[-1.,0.,5.,10.,11.,float('nan')][(case//8)%6];radius=[10.,0.,-5.][(case//48)%3]
 u.mem_write(actor,bytes(0x7e0));u.mem_write(actor+0x44,struct.pack('<f',radius));u.mem_write(actor+0x1f0,pack(active));u.mem_write(game+4,pack(array,present))
 u.mem_write(frame+8,struct.pack('<f',distance));u.mem_write(frame-8,pack(cell));u.mem_write(frame-0x68,struct.pack('<4f',0.,0.,0.,1.));u.mem_write(frame-0x58,struct.pack('<4f',1.,0.,0.,1.))
 u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_ESP,frame-0xb4);u.reg_write(UC_X86_REG_EBX,actor);u.reg_write(UC_X86_REG_EDI,game);u.reg_write(UC_X86_REG_ESI,0);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.reg_write(UC_X86_REG_EAX,cell)
 casts=0;u.emu_start(0x81d3d0,0x81d44c,count=10000)
 out.extend(pack(active,occluded,present));out.extend(struct.pack('<2f',distance,radius));out.extend(pack(casts,read(actor+0x1f0)))
 for i in range(5):out.extend(u.mem_read(actor+0x210+i*128,4))
p=root/'tests/fixtures/beta-visibility.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
