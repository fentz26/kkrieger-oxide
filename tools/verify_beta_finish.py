#!/usr/bin/env python3
"""Trace released commit/fallback tails with a controlled zone callback.

SkipCell is null: cleanup of an owned skip cell remains collision-driver work.
"""
import hashlib,pathlib,random,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_EIP
root=pathlib.Path(__file__).resolve().parent.parent
assert hashlib.sha256((root/'.reference/pno0001.exe').read_bytes()).hexdigest()=='ac2192b740c57125067a29d2e0ea6d95b222543398188c5f67bbf01157622685'
image=(root/'.reference/beta-image.bin').read_bytes()
assert hashlib.sha256(image).hexdigest()=='148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
base=0x7c0000
u=Uc(UC_ARCH_X86,UC_MODE_32);u.mem_map(base,(len(image)+4095)&~4095);u.mem_write(base,image)
u.mem_map(0x3000000,0x10000);u.mem_map(0x5000000,0x1000)
particle,frame,game=0x3000000,0x3008000,0x3004000
read=lambda p:struct.unpack('<I',u.mem_read(p,4))[0]
pack=lambda *v:struct.pack('<'+'I'*len(v),*v)
called=False;callback_args=bytes(40)
def hook(u,address,size,user):
    global called,callback_args
    if address!=0x81b15a:return
    esp=u.reg_read(UC_X86_REG_ESP)
    assert u.reg_read(UC_X86_REG_ECX)==game
    start,old_cell,end,new_cell,p=struct.unpack('<5I',u.mem_read(esp+4,20))
    assert p==particle and start==particle+0x50
    called=True;callback_args=bytes(u.mem_read(start,16))+bytes(u.mem_read(end,16))+pack(old_cell,new_cell)
    u.mem_write(particle+0x50,changes[:16]);u.mem_write(particle+0x40,changes[16:32])
    u.mem_write(particle+0x60,changes[32:48]);u.mem_write(end,changes[48:64])
    u.mem_write(particle+0x1c,changes[64:68])
    u.reg_write(UC_X86_REG_EIP,read(esp));u.reg_write(UC_X86_REG_ESP,esp+24)
u.hook_add(UC_HOOK_CODE,hook)
rng=random.Random(0x200412)
out=bytearray(pack(256))
for case in range(256):
    raw=[rng.getrandbits(32) for _ in range(32)]
    raw[3]=case%5;raw[7]=0 if (case//7)%2==0 else 0x12345678;raw[8]=0;raw[9]=0
    original=pack(*raw)
    target=pack(*[rng.getrandbits(32) for _ in range(4)])
    next_cell=0 if (case//11)%2==0 else 0x87654321
    mode=(case//3)%2;completion=case%3
    changes=pack(*[rng.getrandbits(32) for _ in range(16)],0xabcdef01)
    u.mem_write(particle,original);u.mem_write(frame-0x1c,target)
    u.mem_write(frame-0xc,pack(next_cell));u.mem_write(frame-8,pack(game))
    u.mem_write(frame,pack(0,0x5000000,particle,mode))
    u.mem_write(frame+0xb,bytes([int(completion==1)]))
    u.reg_write(UC_X86_REG_EBP,frame);u.reg_write(UC_X86_REG_EBX,particle);u.reg_write(UC_X86_REG_ESP,frame-0xf0)
    called=False;callback_args=bytes(40)
    u.emu_start(0x81baa5 if completion==2 else 0x81ba4e,0x5000000,count=1000)
    assert u.reg_read(UC_X86_REG_ESP)==frame+16
    status=u.reg_read(UC_X86_REG_EAX)&255
    assert status==completion and called==(raw[3]!=0)
    out.extend(original);out.extend(target);out.extend(pack(next_cell,mode,completion));out.extend(changes)
    out.extend(u.mem_read(particle,128));out.extend(pack(status,int(called)));out.extend(callback_args)
path=root/'tests/fixtures/beta-finish.bin';path.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
