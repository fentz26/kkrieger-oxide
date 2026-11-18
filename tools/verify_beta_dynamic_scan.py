#!/usr/bin/env python3
"""Run full candidate scan with actual hull, no-FX Particle::Hit and PartBox stub.

Hit-effect operator slots remain null; shot selection/Stop execute original code.
Hooks observe callback arguments without replacing any callee.
"""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x40000);u.mem_map(0x3100000,0x20000);u.mem_map(0x5000000,0x1000)
game=0x3100000
ptrs=[0x3000000+i*0x400 for i in range(6)];ptrs[4]=game+0x14d44
lookup={p:i for i,p in enumerate(ptrs)}
static,add,sublist,walk,primary,other,event,direction,end,start,output,stack=0x3004000,0x3010000,0x3011000,0x3012000,0x3020000,0x3020100,0x3021000,0x3022000,0x3023000,0x3024000,0x3025000,0x3039000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
vec=lambda p,n:list(struct.unpack('<'+'I'*n,u.mem_read(p,4*n)))
fword=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
active=-1;trace=[]
def hook(u,address,size,user):
    global active
    esp=u.reg_read(UC_X86_REG_ESP)
    if address==0x81a258:active=lookup.get(read(esp+16),-1)
    elif address==0x801000:raise AssertionError('unexpected event allocation')
    elif address==0x81a98d:
        assert active>=0 and u.reg_read(UC_X86_REG_ECX)==first_ptr
        g,point,plane,distance,monster=struct.unpack('<5I',u.mem_read(esp+4,20))
        assert g==game and distance==0
        shot=read(first_ptr+0x24)
        record=[1,active,first_id,read(first_ptr+0xc)]+vec(point,4)+vec(plane,4)+[monster,distance,read(shot+0x30) if shot else 0xffffffff,int(shot!=0)]
        trace.append(record)
    elif address==0x81ab9a:
        assert active>=0 and u.reg_read(UC_X86_REG_ECX)==0x3026000+active*0x10
        point,delta,mass=struct.unpack('<3I',u.mem_read(esp+4,12))
        record=[2,active,active+20]+vec(point,4)+vec(delta,3)+[mass]
        trace.append(record+[0]*(16-len(record)))
u.hook_add(UC_HOOK_CODE,hook)
rng=random.Random(0x200418)
shapes=[(1.5,0.5),(0.5,0.5),(0.,1.),(-0.5,1.),(-1.,1.),(0.,0.5)]
out=bytearray(pack(256));counts=[0,0];max_callbacks=0
for case in range(256):
    pk=[None,0,1,2,3,8][(case//8)%6]
    mass_bits=[fword(0.),fword(1.),fword(-1.),0x7fc01234][(case//3)%4]
    first_present=(case//2)%2!=0;alias=first_present and pk is not None and case%7==0
    fk=pk if alias else [1,3,8,35][(case//5)%4]
    first_id=3 if alias else 7
    first_ptr=(primary if alias else other) if first_present else 0
    shot_present=first_present and fk!=35 and case%3!=0
    select=[0,1,0xffffffff][(case//7)%3]
    reflect=(case//11)%2;monster_mode=(case//13)%2;interrupt=(case//17)%2;static_present=case%5==0
    u.mem_write(primary,bytes(128));u.mem_write(other,bytes(128));u.mem_write(event,bytes(180))
    if pk is not None:u.mem_write(primary+0xc,pack(pk));u.mem_write(primary+0x2c,pack(mass_bits))
    if first_present:
        u.mem_write(first_ptr+0xc,pack(fk));u.mem_write(first_ptr+0x24,pack(event if shot_present else 0))
    u.mem_write(event+8,pack(1,100));u.mem_write(event+0x30,pack(select))
    u.mem_write(game+0x15108,bytes(0x100))
    if first_present:u.mem_write(game+0x15108+fk*4,pack(2 if reflect else 0))
    u.mem_write(game+0x15540,pack(12345));u.mem_write(game+0x15538,pack(monster_mode))
    modes=[1,17 if case%4==0 else 1,1,9 if case%3==0 else 1,1,33 if case%6==0 else 1]
    only=[case%2,0,0,(case//3)%2,0,0];monsters=[0,0,1,0,0,1]
    boxes=[(case+i)%3==0 for i in range(6)]
    initial_masks=[rng.getrandbits(32) for _ in range(6)];geometry=[]
    for i,(center,extent) in enumerate(shapes):
        planes=[[1.,0.,0.,extent-center],[-1.,0.,0.,extent+center],[0.,1.,0.,1.],[0.,-1.,0.,1.],[0.,0.,1.,1.],[0.,0.,-1.,1.]]
        data=struct.pack('<24f',*[v for p in planes for v in p]);geometry.append(data)
        p=ptrs[i];u.mem_write(p,bytes(0x2a4));u.mem_write(p+0x20,data);u.mem_write(p+0xf0,pack(6))
        u.mem_write(p+0x100,pack(modes[i],only[i]));u.mem_write(p+0x120,pack(initial_masks[i]))
        u.mem_write(p+0x19c,pack(0x3026000+i*0x10 if boxes[i] else 0,0x3027000+i*0x10 if monsters[i] else 0))
    static_planes=[[1.,0.,0.,2.5],[-1.,0.,0.,-2.],[0.,1.,0.,1.],[0.,-1.,0.,1.],[0.,0.,1.,1.],[0.,0.,-1.,1.]]
    u.mem_write(static,bytes(0x140));u.mem_write(static+0x20,struct.pack('<24f',*[v for p in static_planes for v in p]));u.mem_write(static+0xf0,pack(6));u.mem_write(static+0x100,pack(1))
    u.mem_write(add+0x138,pack(sublist,1));u.mem_write(sublist,pack(static));u.mem_write(walk,pack(add))
    u.mem_write(game+0x14044,pack(*ptrs));u.mem_write(game+0x1484c,pack(6))
    p0=[-3.,0.,0.,1.];p1=[3.,0.,0.,1.];d=[1.,0.,0.,0.]
    if case%19==0:p0=[0.,0.,0.,1.]
    if case%23==0:p0=[-3.,3.,0.,1.];p1=[3.,3.,0.,1.]
    if case%29==0:p0=[-3.,-3.,0.,1.];p1=[3.,3.,0.,1.];d=[0.5**0.5,0.5**0.5,0.,0.]
    vectors=struct.pack('<12f',*d,*p1,*p0)
    prior=pack(*[rng.getrandbits(32) for _ in range(4)],fword(-100.))
    u.mem_write(direction,vectors[:16]);u.mem_write(end,vectors[16:32]);u.mem_write(start,vectors[32:]);u.mem_write(output,prior)
    u.mem_write(0x14a0980,pack(interrupt))
    u.mem_write(stack,pack(0x5000000,direction,end,start,walk,int(static_present),output,output+16,primary if pk is not None else 0,first_ptr))
    u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game);u.reg_write(UC_X86_REG_FPCW,0x143f)
    active=-1;trace=[]
    u.emu_start(0x81b213,0x5000000,count=30000)
    assert u.reg_read(UC_X86_REG_ESP)==stack+40 and len(trace)<=12
    hit=u.reg_read(UC_X86_REG_EAX)&255;assert hit in [0,1];counts[hit]+=1;max_callbacks=max(max_callbacks,len(trace))
    out.extend(vectors);out.extend(pack(int(pk is not None),pk or 0,mass_bits,int(first_present),first_id,fk,int(shot_present),select,reflect,monster_mode,interrupt,int(static_present)));out.extend(prior)
    for i in range(6):out.extend(pack(modes[i],only[i],monsters[i],int(i==4),i+20 if boxes[i] else 0xffffffff,initial_masks[i]));out.extend(geometry[i])
    out.extend(pack(hit,read(0x14a0980),int(shot_present),read(event+0x30) if shot_present else 0));out.extend(u.mem_read(output,20))
    out.extend(pack(*[read(p+0x120) for p in ptrs],len(trace)))
    for record in trace+[[0]*16]*(12-len(trace)):out.extend(pack(*record))
path=root/'tests/fixtures/beta-dynamic-scan.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest(),'miss_hit',counts,'maximum_callbacks',max_callbacks)
