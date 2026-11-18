#!/usr/bin/env python3
"""Bounded Unicorn trace of the released beta's one-word IPP viewport wrapper.

Callees are controlled stubs: this proves dispatch/state order, not rendering.
"""
import hashlib,json,pathlib,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EIP,UC_X86_REG_ECX,UC_X86_REG_EDX,UC_X86_REG_EAX
root=pathlib.Path(__file__).resolve().parent.parent;ref=root/'.reference'
exe=(ref/'pno0001.exe').read_bytes();assert hashlib.sha256(exe).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(ref/'beta-image.bin').read_bytes();base=0x7c0000;start=0x814683;end=0x8146ff
assert hashlib.sha256(image[start-base:end-base]).hexdigest()=='0bfb530be2448fb222ffbed07b241af24adce77bce765ff89b1bf1f6dd84480d'
word=lambda v:struct.pack('<I',v&0xffffffff)
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000);u.mem_map(0x4000000,0x10000);u.mem_map(0x5000000,0x1000)
parent,manager,child,target,environment,game=0x3000000,0x3001000,0x3004000,0x3005000,0x3006000,0x3007000
read=lambda a:struct.unpack('<I',u.mem_read(a,4))[0]
trace=[];cached=False;size=0;count=0
stubs={0x81431e:'find',0x80e106:'input',0x818d42:'execute',0x8142e1:'allocate',0x814358:'viewport',0x8151b3:'paint'}
def hook(u,address,size_,user):
 if address not in stubs:return
 name=stubs[address];esp=u.reg_read(UC_X86_REG_ESP);ecx=u.reg_read(UC_X86_REG_ECX);edx=u.reg_read(UC_X86_REG_EDX)
 result=0;pop=0
 if name=='find':
  assert ecx==manager and read(esp+4)==parent;pop=4;result=target if cached else 0
 elif name=='input':
  assert ecx==parent and read(esp+4)==0;pop=4;result=child
 elif name=='execute':
  assert ecx==child and read(esp+4)==environment;pop=4
 elif name=='allocate':
  assert ecx==manager and [read(esp+i) for i in [4,8,12]]==[parent,size,count&0xffffffff];pop=12;result=target
 elif name=='viewport':
  assert ecx==manager and read(esp+4)==target;pointer=read(esp+8);pop=8
  u.mem_write(pointer,b''.join(word(v) for v in [0xffffffff,17,2,3,130,67,0x12345678,0xabcdef01]))
 elif name=='paint':
  assert ecx==environment and read(esp+4)==game;pop=4
  assert [read(edx+i*4) for i in range(8)]==[0xffffffff,17,2,3,130,67,0x12345678,0]
 trace.append(name)
 u.reg_write(UC_X86_REG_EAX,result);u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+4+pop)
u.hook_add(UC_HOOK_CODE,hook)
results=[]
for cached in [False,True]:
 for size in range(4):
  for count in [-1,1,4]:
   trace=[];stack=0x400f000
   u.mem_write(0x140e128,word(manager));u.mem_write(manager+0x1864,word(game));u.mem_write(manager+0xe4,word(0))
   u.mem_write(stack,b''.join(word(v) for v in [0x5000000,parent,environment,size,count]));u.reg_write(UC_X86_REG_ESP,stack)
   u.emu_start(start,0x5000000,count=1000)
   assert u.reg_read(UC_X86_REG_EIP)==0x5000000 and u.reg_read(UC_X86_REG_ESP)==stack+20
   expected=['find'] if cached else ['find','input','execute','allocate','viewport','paint']
   assert trace==expected and read(manager+0xe4)==target
   results.append({'cached':cached,'size':size,'output_count':count,'calls':trace})
report={'entry':hex(start),'code_sha256':hashlib.sha256(image[start-base:end-base]).hexdigest(),'cases':results,'scope':'Exact released viewport wrapper call order and LastOutput store; callees, camera effects, GPU output and complete handler ABI remain separate.'}
(ref/'beta-viewport-evidence.json').write_text(json.dumps(report,indent=2)+'\n')
print(f'{len(results)} released viewport wrapper traces passed; code SHA256 {report["code_sha256"]}')

# Execute PrepareViewport and both viewport initializers without stubbing them.
# Device screen dimensions are supplied as data; screen fallback overwrites them.
v=Uc(UC_ARCH_X86,UC_MODE_32);v.mem_map(base,(len(image)+4095)&~4095);v.mem_write(base,image)
v.mem_map(0x3000000,0x10000);v.mem_map(0x4000000,0x10000);v.mem_map(0x5000000,0x1000)
v.mem_write(0x83f868,word(0x3008000));v.mem_write(0x3008070,word(1280)+word(1024))
for size in range(6):
 v.mem_write(target,word(0x3009000)+word(0)+word(size)+word(1));v.mem_write(0x3009020,word(37))
 pointer=0x300a000;stack=0x400f000
 v.mem_write(pointer,b'\xcc'*32);v.mem_write(stack,word(0x5000000)+word(target)+word(pointer))
 v.reg_write(UC_X86_REG_ESP,stack);v.emu_start(0x814358,0x5000000,count=1000)
 actual=list(struct.unpack('<8I',v.mem_read(pointer,32)))
 expected=[0xffffffff,37,0,0,0,0,3,0xff000000] if size<4 else [0,0xffffffff,0,128,1024,640,3,0x0ff00000]
 assert actual==expected,(size,actual)
 assert v.reg_read(UC_X86_REG_ESP)==stack+12
print('6 actual PrepareViewport/initializer traces passed')
