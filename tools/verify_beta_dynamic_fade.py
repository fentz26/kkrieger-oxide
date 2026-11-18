#!/usr/bin/env python3
"""Execute original dynamic matrix Fade and all vector math callees."""
import runpy,pathlib,struct,random,hashlib
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
rng=random.Random(0x200429);out=bytearray(pack(128));first,second,result=0x3004000,0x3004100,0x3004200
for case in range(128):
 matrices=[]
 for _ in range(2):
  m=[rng.uniform(-2.,2.) for _ in range(16)];m[3]=m[7]=m[11]=0.;m[15]=1.;matrices.append(m)
 factor=[0.,1.,0.5,-0.25,1.25][case%5]
 inputs=struct.pack('<33f',*[v for m in matrices for v in m],factor)
 u.mem_write(first,inputs[:64]);u.mem_write(second,inputs[64:128]);u.mem_write(stack,pack(0x5000000,first,second)+inputs[128:])
 u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,result);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x80659e,0x5000000,count=10000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+16
 out.extend(inputs);out.extend(u.mem_read(result,64))
p=root/'tests/fixtures/beta-dynamic-fade.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
