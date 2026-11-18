#!/usr/bin/env python3
"""Execute released inverse-and-multiply pair used by dynamic physics."""
import runpy,pathlib,struct,random,hashlib
state=runpy.run_path(str(pathlib.Path(__file__).with_name('verify_beta_runtime_step.py')))
globals().update({k:v for k,v in state.items() if not k.startswith('__')})
rng=random.Random(0x200426);out=bytearray(pack(128));inverse=0x3004000;current=0x3004100;result=0x3004200
for case in range(128):
 matrices=[]
 for _ in range(2):
  m=[rng.uniform(-2.,2.) for _ in range(16)];m[3]=m[7]=m[11]=0.;m[15]=1.;matrices.append(m)
 if case<4:
  matrices[0]=[2.,0.,0.,0.,0.,3.,0.,0.,0.,0.,-0.5,0.,1.,-2.,3.,1.]
  matrices[1]=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.]
 inputs=struct.pack('<32f',*[v for m in matrices for v in m]);u.mem_write(inverse,inputs[:64]);u.mem_write(current,inputs[64:])
 u.mem_write(stack,pack(0x5000000));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,inverse);u.reg_write(UC_X86_REG_FPCW,0x143f)
 u.emu_start(0x8064ec,0x5000000,count=5000)
 u.mem_write(stack,pack(0x5000000,inverse,current));u.reg_write(UC_X86_REG_ESP,stack);u.reg_write(UC_X86_REG_ECX,result)
 u.emu_start(0x805c2c,0x5000000,count=5000)
 assert u.reg_read(UC_X86_REG_ESP)==stack+12
 out.extend(inputs);out.extend(u.mem_read(result,64))
p=root/'tests/fixtures/beta-motion-delta.bin';p.write_bytes(out)
print(len(out),hashlib.sha256(out).hexdigest())
