"""Bounded self-contained i386 COFF execution for offline reference algorithms."""
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_FPCW

class Oracle:
    def __init__(self,path):
        blob=path.read_bytes()
        machine,count,_,symbols,n_symbols,opt_size,_=struct.unpack_from('<HHIIIHH',blob)
        assert machine==0x14c
        self.mu=Uc(UC_ARCH_X86,UC_MODE_32)
        self.symbols={}
        base=0x400000
        assert count<128
        sections=[]
        self.mu.mem_map(base,max(1,count)*0x10000)
        for i in range(count):
            off=20+opt_size+i*40
            _,_,size,raw,relocs,_,n_relocs,_,_=struct.unpack_from('<IIIIIIHHI',blob,off+8)
            assert size<=0x10000
            address=base+i*0x10000
            sections.append((address,size,raw,relocs,n_relocs))
            if size and raw:self.mu.mem_write(address,blob[raw:raw+size])
        strings=symbols+n_symbols*18
        values=[];defined=[];i=0
        while i<n_symbols:
            off=symbols+i*18;name=blob[off:off+8]
            if name[:4]==b'\0'*4:
                pos=strings+struct.unpack_from('<I',name,4)[0];name=blob[pos:].split(b'\0')[0]
            else:name=name.rstrip(b'\0')
            value,section,_,_,aux=struct.unpack_from('<IhHBB',blob,off+8)
            address=sections[section-1][0]+value if section>0 else value
            values.append(address);defined.append(section>0)
            if section>0:self.symbols[name.decode()]=address
            values.extend([0]*aux);defined.extend([False]*aux);i+=1+aux
        for address,_,_,relocs,n in sections:
            for i in range(n):
                off,symbol,kind=struct.unpack_from('<IIH',blob,relocs+i*10)
                assert defined[symbol],f'unresolved symbol index {symbol}'
                location=address+off;addend=struct.unpack('<I',self.mu.mem_read(location,4))[0]
                assert kind in (6,20),f'unsupported relocation {kind}'
                value=addend+values[symbol]-(location+4 if kind==20 else 0)
                self.mu.mem_write(location,struct.pack('<I',value&0xffffffff))
        self.stack=0x100000;self.stop=0x200000;self.data=0x2000000
        self.mu.mem_map(self.stack,0x10000);self.mu.mem_map(self.stop,0x1000);self.mu.mem_map(self.data,0x100000)
        self.mu.reg_write(UC_X86_REG_FPCW,0x37f)

    def call(self,symbol,*arguments):
        esp=self.stack+0xff00
        self.mu.mem_write(esp,struct.pack('<'+'I'*(len(arguments)+1),self.stop,*arguments))
        self.mu.reg_write(UC_X86_REG_ESP,esp)
        self.mu.emu_start(self.symbols[symbol],self.stop,timeout=2_000_000,count=2_000_000)
        assert self.mu.reg_read(UC_X86_REG_ESP)==esp+4,'oracle did not return'
        assert self.mu.reg_read(UC_X86_REG_FPCW)==0x37f,'oracle failed to restore x87 control'
