"""Assemble self-contained Intel x86 source and relocate its ELF text section."""
import pathlib,struct,subprocess,tempfile

def assemble(source,address):
 with tempfile.TemporaryDirectory() as tmp:
  tmp=pathlib.Path(tmp);asm=tmp/'native.s';obj=tmp/'native.o'
  asm.write_text('.intel_syntax noprefix\n.text\n'+source+'\n')
  subprocess.run(['clang','-target','i386-unknown-linux-gnu','-c',str(asm),'-o',str(obj)],check=True)
  elf=obj.read_bytes()
 shoff=struct.unpack_from('<I',elf,32)[0];size,count,strings=struct.unpack_from('<HHH',elf,46)
 headers=[struct.unpack_from('<10I',elf,shoff+i*size) for i in range(count)]
 h=headers[strings];names=elf[h[4]:h[4]+h[5]]
 text_index=next(i for i,h in enumerate(headers) if names[h[0]:].split(b'\0')[0]==b'.text')
 text=headers[text_index];code=bytearray(elf[text[4]:text[4]+text[5]]);symbols={}
 def symtab(index):
  h=headers[index];return [struct.unpack_from('<IIIBBH',elf,p) for p in range(h[4],h[4]+h[5],h[9])]
 for i,h in enumerate(headers):
  if h[1]==2:
   strings=headers[h[6]];strings=elf[strings[4]:strings[4]+strings[5]]
   for s in symtab(i):
    if s[5]==text_index:symbols[strings[s[0]:].split(b'\0')[0].decode()]=address+s[1]
  if h[1]==9 and h[7]==text_index:
   syms=symtab(h[6])
   for p in range(h[4],h[4]+h[5],h[9]):
    offset,info=struct.unpack_from('<II',elf,p);s=syms[info>>8];kind=info&255
    assert s[5]==text_index,(kind,s)
    value=struct.unpack_from('<I',code,offset)[0]+address+s[1]
    if kind==2:value-=address+offset
    else:assert kind==1,kind
    struct.pack_into('<I',code,offset,value&0xffffffff)
 return bytes(code),symbols
