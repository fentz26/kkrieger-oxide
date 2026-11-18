#!/usr/bin/env python3
"""Run complete released response; control casts and event/zone callbacks only."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_EIP,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000);u.mem_map(0x5000000,0x1000)
particle,owned,game,stack=0x3000000,0x3001000,0x3004000,0x300f000
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
vec=lambda p,n:list(struct.unpack('<'+'I'*n,u.mem_read(p,4*n)))
trace=[];cast_index=0

def hook(u,address,size,user):
    global cast_index
    if address not in [0x81b45a,0x81a98d,0x81b15a]:return
    esp=u.reg_read(UC_X86_REG_ESP);ecx=u.reg_read(UC_X86_REG_ECX)
    skip=read(owned+0x100)
    assert not skip_link or skip&0x10
    if address==0x81b45a:
        direction,end,start,cell,plane,distance,p,first=struct.unpack('<8I',u.mem_read(esp+4,32))
        assert ecx==game and p==particle and first in [0,particle]
        record=[1,int(first!=0)]+vec(direction,3)+vec(end,4)+vec(start,4)+[read(cell),skip,read(0x14a0980)]+vec(particle+0x40,4)+vec(particle+0x60,4)
        plan=plans[cast_index];cast_index+=1
        hit,interrupt,next_cell=plan[:3]
        u.mem_write(cell,pack(next_cell));u.mem_write(0x14a0980,pack(interrupt))
        u.mem_write(plane,pack(*plan[3:7]));u.mem_write(distance,pack(plan[7]))
        result=hit;pop=32
    elif address==0x81a98d:
        g,impact,plane,speed,flag=struct.unpack('<5I',u.mem_read(esp+4,20))
        assert ecx==particle and g==game and speed==0 and flag==0 and shot_link
        record=[2]+vec(impact,4)+vec(plane,4)+[skip]
        u.mem_write(impact,pack(*shot_changes[:4]));u.mem_write(plane+12,pack(shot_changes[4]))
        result=0;pop=20
    else:
        start,old_cell,end,new_cell,p=struct.unpack('<5I',u.mem_read(esp+4,20))
        assert ecx==game and p==particle and start==particle+0x50
        record=[3]+vec(start,4)+vec(end,4)+[old_cell,new_cell,skip]+vec(particle+0x40,4)+vec(particle+0x60,4)
        u.mem_write(start,pack(*zone_changes[:4]));u.mem_write(end,pack(*zone_changes[4:]))
        result=0;pop=20
    assert len(record)<=32
    trace.append(record+[0]*(32-len(record)))
    u.reg_write(UC_X86_REG_EAX,result);u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+4+pop)
u.hook_add(UC_HOOK_CODE,hook)
rng=random.Random(0x200413)
fword=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
out=bytearray(pack(256));statuses=[0,0,0];max_casts=0
for case in range(256):
    raw=[0]*32;raw[3]=case%5;raw[7]=0 if (case//7)%2==0 else 0x12345678
    skip_link=(case//3)%2!=0;shot_link=(case//2)%2!=0
    raw[8]=owned if skip_link else 0;raw[9]=0x2468ace0 if shot_link else 0;raw[11]=fword(1.)
    raw[12:]=[fword(rng.uniform(-3.,3.)) for _ in range(20)]
    if case%8==6:raw[16:19]=raw[20:23]
    mode=(case//5)%2
    plans=[]
    scenario=case%8
    hits={0:[0,0,0],1:[1,0,0],2:[1,1,0],3:[1,1,1],4:[1,0,0],5:[1,1,0],6:[0,0,0],7:[0,0,0]}[scenario]
    n2=[[-1.,0.,0.],[1.,0.,0.],[0.,1.,0.]][(case//8)%3]
    for n,normal in enumerate([[1.,0.,0.],n2,[0.,0.,1.]]):
        interrupt=int((scenario==4 and n==0) or (scenario==5 and n==1) or (not hits[n] and (case//8)%2!=0))
        cell=0 if (case+n)%4==0 else [0x87654321,0x89abcdef,0x9abcdef0][n]
        plans.append([hits[n],interrupt,cell]+[fword(x) for x in normal]+[fword(rng.uniform(-2.,2.)),fword(rng.uniform(-1.,0.))])
    initial_skip=rng.getrandbits(32);initial_interrupt=1
    shot_changes=[fword(rng.uniform(-3.,3.)) for _ in range(5)]
    zone_changes=[fword(rng.uniform(-3.,3.)) for _ in range(8)]
    u.mem_write(particle,pack(*raw));u.mem_write(owned+0x100,pack(initial_skip));u.mem_write(0x14a0980,pack(initial_interrupt))
    u.mem_write(stack,pack(0x5000000,particle,mode));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game)
    u.reg_write(UC_X86_REG_FPCW,0x143f);trace=[];cast_index=0
    u.emu_start(0x81b5bd,0x5000000,count=10000)
    assert u.reg_read(UC_X86_REG_ESP)==stack+12 and len(trace)<=5
    status=u.reg_read(UC_X86_REG_EAX)&255;assert status in [0,1,2]
    statuses[status]+=1;max_casts=max(max_casts,cast_index)
    out.extend(pack(*raw,mode))
    for plan in plans:out.extend(pack(*plan))
    out.extend(pack(initial_skip,initial_interrupt,*shot_changes,*zone_changes))
    out.extend(u.mem_read(particle,128));out.extend(pack(status,read(owned+0x100),read(0x14a0980),len(trace)))
    for record in trace+[[0]*32]*(5-len(trace)):out.extend(pack(*record))
path=root/'tests/fixtures/beta-response.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest(),'statuses',statuses,'max_casts',max_casts)
