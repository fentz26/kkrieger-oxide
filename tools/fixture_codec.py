"""Lossless word-aligned LZ packing for offline golden fixtures, never game data."""
import struct
import sys

MAGIC=b'WZ01'

def pack(data):
    assert len(data)%4==0 and sys.byteorder=='little'
    words=memoryview(data).cast('I');count=len(words);table=[-1]*(1<<18)
    out=bytearray(MAGIC+struct.pack('<I',len(data)))
    def key(i):
        value=words[i]*0x9e3779b1 ^ words[i+1]*0x85ebca6b ^ words[i+2]*0xc2b2ae35 ^ words[i+3]*0x27d4eb2f
        return (value ^ (value>>16) ^ (value>>32))&((1<<18)-1)
    def literal(start,end):
        if end>start:out.extend(struct.pack('<I',end-start));out.extend(data[start*4:end*4])
    position=0;start=0
    while position+4<=count:
        h=key(position);previous=table[h];table[h]=position
        if previous>=0 and words[position:position+4]==words[previous:previous+4]:
            end=position+4;match=previous+4
            while end<count and words[end]==words[match]:end+=1;match+=1
            literal(start,position)
            out.extend(struct.pack('<II',0x80000000|(end-position),position-previous))
            for i in range(max(position+1,end-3),end):
                if i+4<=count:table[key(i)]=i
            position=end;start=end
        else:position+=1
    literal(start,count)
    encoded=bytes(out)
    assert unpack(encoded)==data
    return encoded

def unpack(data):
    assert data[:4]==MAGIC
    expected=struct.unpack_from('<I',data,4)[0];position=8;out=bytearray()
    while position<len(data):
        token=struct.unpack_from('<I',data,position)[0];position+=4;length=token&0x7fffffff
        assert length>0
        if token>>31:
            distance=struct.unpack_from('<I',data,position)[0];position+=4
            assert 0<distance<=len(out)//4
            start=len(out)-distance*4
            if length<=distance:out.extend(out[start:start+length*4])
            else:
                pattern=bytes(out[start:]);out.extend((pattern*((length+distance-1)//distance))[:length*4])
        else:out.extend(data[position:position+length*4]);position+=length*4
        assert len(out)<=expected
    assert position==len(data) and len(out)==expected
    return bytes(out)
