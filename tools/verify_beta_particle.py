#!/usr/bin/env python3
"""Trace released PhysicPart, controlling only its three world boundaries."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_EIP,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000);u.mem_map(0x3100000,0x20000);u.mem_map(0x5000000,0x1000)
body,point,stack,game=0x3000000,0x3008000,0x300f000,0x3100000
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
rng=random.Random(0x200409)
running=False
force_only=False
trace=[];collision_input=bytes(128)
def hook(u,address,size,user):
    global collision_input
    if force_only and address==0x81c341:
        esp=u.reg_read(UC_X86_REG_ESP)
        force_visits.append(read(esp+4))
        u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+8)
        return
    if not running or address not in [0x819fea,0x81b115,0x81b5bd]:return
    esp=u.reg_read(UC_X86_REG_ESP);ecx=u.reg_read(UC_X86_REG_ECX)
    if address==0x819fea:
        trace.append(1);assert ecx==cell and read(esp+8)==0
        assert u.mem_read(read(esp+4),16)==old_collision
        result=outside;pop=8
    elif address==0x81b115:
        trace.append(2);assert ecx==game
        assert u.mem_read(esp+4,16)==old_collision
        result=found;pop=16
    else:
        trace.append(3);assert ecx==game and read(esp+4)==particle and read(esp+8)==0
        collision_input=bytes(u.mem_read(particle,128))
        u.mem_write(particle+0x40,response);u.mem_write(particle+0x64,response_y)
        result=collided;pop=8
    u.reg_write(UC_X86_REG_EAX,result);u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+4+pop)
u.hook_add(UC_HOOK_CODE,hook)
out=bytearray(pack(256))
for case in range(256):
    running=False
    kind=[1,3,5][case%3];index=case%5;particle=body+index*128
    cell=0 if case%3==0 else 0x12345678
    u.mem_write(body,bytes([0xa5])*0x600);u.mem_write(point,struct.pack('<4f',0.,0.,0.,1.))
    u.mem_write(stack,pack(0x5000000,point,cell,kind))
    u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,body);u.reg_write(UC_X86_REG_FPCW,0x143f)
    u.emu_start(0x81c9c1,0x5000000,count=100000)
    for p in range(5):
        address=body+p*128
        u.mem_write(address+0x40,struct.pack('<16f',*[rng.uniform(-10.,10.) for _ in range(16)]))
        u.mem_write(address+0x30,struct.pack('<4f',*[rng.uniform(-1.,1.) for _ in range(4)]))
        u.mem_write(address+0x28,struct.pack('<f',rng.uniform(-0.2,1.2)))
    edges=[0,0x80000000,1,0x80000001,0x007fffff,0x807fffff,0x00800000,0x80800000]
    if case<len(edges):
        for p in range(5):u.mem_write(body+p*128+0x30,pack(*([edges[case]]*4)))
    if case==17:u.mem_write(particle+0x30,struct.pack('<f',-float('inf')))
    u.mem_write(particle+0x14,pack(case%2))
    old_collision=bytes(u.mem_read(particle+0x50,16))
    gravity,player_gravity,air,ground=[rng.uniform(0.,0.2),rng.uniform(0.,0.2),rng.uniform(0.,1.2),rng.uniform(0.,1.2)]
    fly=int(case%3==0);on_ground=int(case%2==0)
    settings=struct.pack('<4f',gravity,player_gravity,air,ground)
    u.mem_write(game+0x15390,settings);u.mem_write(game+0x15360,pack(fly,on_ground))
    outside=int(case%4==0);found=0 if case%7==0 else 0x87654321;collided=case%2
    response=struct.pack('<4f',*[rng.uniform(-20.,20.) for _ in range(4)])
    if case<len(edges):response=struct.pack('<4f',0.,-0.,1.,-1.)
    if case==17:response=struct.pack('<f',float('inf'))+response[4:]
    response_y=struct.pack('<f',rng.uniform(-10.,10.))
    inputs=bytes(u.mem_read(body,640))
    trace=[];collision_input=bytes(128);running=True
    u.mem_write(stack,pack(0x5000000,particle));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game)
    u.reg_write(UC_X86_REG_FPCW,0x143f)
    u.emu_start(0x81c341,0x5000000,count=5000)
    assert u.reg_read(UC_X86_REG_ESP)==stack+8
    out.extend(inputs);out.extend(pack(kind,index));out.extend(settings)
    out.extend(pack(fly,on_ground,outside,found,collided));out.extend(response);out.extend(response_y)
    out.extend(u.mem_read(body,640));out.extend(pack(len(trace),*(trace+[0]*3)[:3]));out.extend(collision_input)
    # Execute the real Physic pass with particle updates controlled as no-ops;
    # its final force-add/clear phase and vector math are the original code.
    running=False;force_only=True;force_visits=[]
    u.mem_write(game+0x14844,pack(1,0,0));u.mem_write(game+0x44,pack(body))
    u.mem_write(stack,pack(0x5000000,0x3f800000))
    u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game)
    u.emu_start(0x81c728,0x5000000,count=5000)
    assert force_visits==[body+p*128 for p in range(5)]
    assert u.reg_read(UC_X86_REG_ESP)==stack+8
    out.extend(u.mem_read(body,640));force_only=False
path=root/'tests/fixtures/beta-particle.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
