#!/usr/bin/env python3
"""Run released additive traversal with real hull tests; stub only CollideRaySub."""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_EIP,UC_X86_REG_FPCW
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x40000);u.mem_map(0x5000000,0x1000)
cells,lists,game,particle,end,start,direction,current,output,stack=0x3000000,0x3010000,0x3020000,0x3021000,0x3022000,0x3022100,0x3022200,0x3022300,0x3022400,0x303f000
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
vec=lambda p,n:struct.unpack('<'+'I'*n,u.mem_read(p,4*n))
fword=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
visited_trace=[];sub_calls=0

def hook(u,address,size,user):
    global visited_trace,sub_calls
    if address!=0x81b213:return
    esp=u.reg_read(UC_X86_REG_ESP)
    d,e,s,visited,count,plane,dist,p,first=struct.unpack('<9I',u.mem_read(esp+4,36))
    assert u.reg_read(UC_X86_REG_ECX)==game
    assert d==direction and e==end and s==start and p==p_arg and first==first_arg
    visited_trace=[(ptr-cells)//0x200 for ptr in vec(visited,count)]
    assert all(0<=i<ncell for i in visited_trace) and 1<=count<=16
    sub_calls+=1
    u.mem_write(plane,sub_result[:16]);u.mem_write(dist,sub_result[16:])
    u.reg_write(UC_X86_REG_EAX,int(sub_hit));u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+40)
u.hook_add(UC_HOOK_CODE,hook)
rng=random.Random(0x200415)
out=bytearray(pack(128));maximum=0
for case in range(128):
    ncell=24 if case%8==0 else 6
    mode=case%8
    neighbors=[];shot=[]
    for i in range(ncell):
        links=([i-1] if i else [])+([i+1] if i+1<ncell else [])
        if mode==3:links=[i]+links
        if mode==4:links=[]
        neighbors.append(links);shot.append(int(mode==2 and i==1))
        planes=[[1.,0.,0.,1.-2*i],[-1.,0.,0.,1.+2*i],
            [0.,1.,0.,1.],[0.,-1.,0.,1.],[0.,0.,1.,1.],[0.,0.,-1.,1.]]
        u.mem_write(cells+i*0x200,bytes(0x140))
        u.mem_write(cells+i*0x200+0x20,struct.pack('<24f',*[v for p in planes for v in p]))
        u.mem_write(cells+i*0x200+0xf0,pack(6));u.mem_write(cells+i*0x200+0x104,pack(shot[-1]))
        u.mem_write(cells+i*0x200+0x128,pack(lists+i*0x80,len(links)))
        if links:u.mem_write(lists+i*0x80,pack(*[cells+j*0x200 for j in links]))
    kind=[None,0,1,2,3,4][(case//8)%6]
    p_arg=0 if kind is None else particle;first_arg=p_arg if case%2 else 0
    u.mem_write(particle+0xc,pack(kind or 0))
    target_x=45. if ncell==24 else [0.,9.,5.,7.,9.,3.,11.,-5.][mode]
    target=struct.pack('<4f',target_x,0.,0.,1.);origin=struct.pack('<4f',0.,0.,0.,1.)
    d=struct.pack('<4f',-1. if target_x<0 else 1.,0.,0.,0.)
    sub_hit=(case//2+case//8)%2!=0
    distance=[-100.,-0.25,0.,-2.][(case//16)%4]
    if case==0:sub_hit=True;distance=-14.
    sub_result=struct.pack('<5f',0.,0.,1.,rng.uniform(-3.,3.),distance)
    u.mem_write(end,target);u.mem_write(start,origin);u.mem_write(direction,d);u.mem_write(current,pack(cells));u.mem_write(output,bytes([0xa5])*20)
    u.mem_write(stack,pack(0x5000000,direction,end,start,current,output,output+16,p_arg,first_arg))
    u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,game);u.reg_write(UC_X86_REG_FPCW,0x143f)
    visited_trace=[];sub_calls=0
    u.emu_start(0x81b45a,0x5000000,count=30000)
    assert u.reg_read(UC_X86_REG_ESP)==stack+36 and sub_calls==1
    final=(read(current)-cells)//0x200
    hit=u.reg_read(UC_X86_REG_EAX)&255;assert hit in [0,1]
    maximum=max(maximum,len(visited_trace))
    out.extend(pack(ncell,0,0xffffffff if kind is None else kind,int(first_arg!=0),int(sub_hit)))
    out.extend(target);out.extend(origin);out.extend(d);out.extend(sub_result)
    for links,only in zip(neighbors,shot):out.extend(pack(only,len(links),*(links+[0]*4)[:4]))
    out.extend(pack(final,hit));out.extend(u.mem_read(output,20))
    out.extend(pack(len(visited_trace),*(visited_trace+[0]*16)[:16]))
path=root/'tests/fixtures/beta-world-cast.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest(),'maximum_visited',maximum)
