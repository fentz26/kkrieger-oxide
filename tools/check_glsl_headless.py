#!/usr/bin/env python3
"""Check KGLS shader catalogues and pixel readback on hardware OpenGL via EGL.

Uses Python's standard library and the installed EGL/GL provider. Software EGL
devices and known software renderers are excluded. Shader catalogues come from
shader-check --export=PATH; at least one hardware context must pass every supplied
catalogue. This tests compilation/linking, not a rendered game-frame comparison.
"""
import ctypes as C,json,sys,struct,pathlib
E=C.CDLL('libEGL.so.1');I=C.c_int;P=C.c_void_p;U=C.c_uint

def bind(name,result,args):
 f=getattr(E,name);f.restype=result;f.argtypes=args;return f
proc=bind('eglGetProcAddress',P,[C.c_char_p])
def ext(name,result,args):
 address=proc(name.encode());assert address,name
 return C.CFUNCTYPE(result,*args)(address)
query=ext('eglQueryDevicesEXT',U,[I,C.POINTER(P),C.POINTER(I)])
platform=ext('eglGetPlatformDisplayEXT',P,[U,P,C.POINTER(I)])
device_string=ext('eglQueryDeviceStringEXT',C.c_char_p,[P,I])
init=bind('eglInitialize',U,[P,C.POINTER(I),C.POINTER(I)])
choose=bind('eglChooseConfig',U,[P,C.POINTER(I),C.POINTER(P),I,C.POINTER(I)])
bindapi=bind('eglBindAPI',U,[U]);context=bind('eglCreateContext',P,[P,P,P,C.POINTER(I)])
pbuffer=bind('eglCreatePbufferSurface',P,[P,P,C.POINTER(I)])
current=bind('eglMakeCurrent',U,[P,P,P,P]);error=bind('eglGetError',I,[])
destroy_context=bind('eglDestroyContext',U,[P,P]);destroy_surface=bind('eglDestroySurface',U,[P,P]);terminate=bind('eglTerminate',U,[P])
devices=(P*16)();count=I();assert query(16,devices,C.byref(count))
print('EGL devices:',count.value,flush=True)
success=False
for device in devices[:count.value]:
 extensions=device_string(device,0x3055) or b''
 if b'EGL_MESA_device_software' in extensions:continue
 node=device_string(device,0x3377);display=platform(0x313f,device,None)
 major,minor=I(),I()
 if not init(display,C.byref(major),C.byref(minor)):
  print('hardware display initialization failed',hex(error()),flush=True);continue
 ctx=surf=None
 try:
  assert bindapi(0x30a2)
  attrs=(I*13)(0x3024,8,0x3023,8,0x3022,8,0x3021,8,0x3033,1,0x3040,8,0x3038)
  config=P();found=I();assert choose(display,attrs,C.byref(config),1,C.byref(found)) and found.value
  ctx=context(display,config,None,(I*1)(0x3038));assert ctx,hex(error())
  surf=pbuffer(display,config,(I*5)(0x3057,16,0x3056,16,0x3038));assert surf,hex(error())
  assert current(display,surf,surf,ctx),hex(error())
  get_string=ext('glGetString',C.c_char_p,[U]);renderer=get_string(0x1f01).decode();vendor=get_string(0x1f00).decode();version=get_string(0x1f02).decode()
  assert not any(x in renderer.lower() for x in ['llvmpipe','softpipe','swrast','swiftshader']),renderer
  ext('glClearColor',None,[C.c_float]*4)(.25,.5,.75,1.)
  ext('glClear',None,[U])(0x4000);ext('glFinish',None,[])()
  pixel=(C.c_ubyte*4)();ext('glReadPixels',None,[I,I,I,I,U,U,P])(0,0,1,1,0x1908,0x1401,pixel)
  assert all(abs(a-b)<=1 for a,b in zip(pixel,[64,128,191,255])),list(pixel)
  create_shader=ext('glCreateShader',U,[U]);shader_source=ext('glShaderSource',None,[U,I,C.POINTER(C.c_char_p),C.POINTER(I)]);compile_shader=ext('glCompileShader',None,[U]);shader_status=ext('glGetShaderiv',None,[U,U,C.POINTER(I)]);shader_log=ext('glGetShaderInfoLog',None,[U,I,C.POINTER(I),P]);delete_shader=ext('glDeleteShader',None,[U])
  create_program=ext('glCreateProgram',U,[]);attach=ext('glAttachShader',None,[U,U]);link=ext('glLinkProgram',None,[U]);program_status=ext('glGetProgramiv',None,[U,U,C.POINTER(I)]);program_log=ext('glGetProgramInfoLog',None,[U,I,C.POINTER(I),P]);delete_program=ext('glDeleteProgram',None,[U]);gl_error=ext('glGetError',U,[])
  for path in sys.argv[1:]:
   data=pathlib.Path(path).read_bytes();assert data[:4]==b'KGLS';total=struct.unpack_from('<I',data,4)[0];offset=8;failures=[];linked=0
   for _ in range(total):
    operator,vslen,fslen=struct.unpack_from('<III',data,offset);offset+=12
    vertex,fragment=data[offset:offset+vslen],data[offset+vslen:offset+vslen+fslen];offset+=vslen+fslen
    shaders=[];program=0
    try:
     for kind,source in [(0x8b31,vertex),(0x8b30,fragment)]:
      shader=create_shader(kind);assert shader;shaders.append(shader);text=C.c_char_p(source);length=I(len(source));shader_source(shader,1,C.byref(text),C.byref(length));compile_shader(shader);ok=I();shader_status(shader,0x8b81,C.byref(ok))
      if not ok.value:
       log=C.create_string_buffer(16384);shader_log(shader,len(log),None,log);raise RuntimeError('compile '+log.value.decode(errors='replace'))
     program=create_program();assert program
     for shader in shaders:attach(program,shader)
     link(program);ok=I();program_status(program,0x8b82,C.byref(ok))
     if not ok.value:
      log=C.create_string_buffer(16384);program_log(program,len(log),None,log);raise RuntimeError('link '+log.value.decode(errors='replace'))
     linked+=1
    except Exception as exc:failures.append((operator,str(exc)))
    finally:
     if program:delete_program(program)
     for shader in shaders:delete_shader(shader)
   assert offset==len(data)
   print(json.dumps({'catalog':pathlib.Path(path).name,'pairs':total,'linked':linked,'failures':failures,'gl_error':gl_error()}),flush=True)
   assert not failures
  print(json.dumps({'vendor':vendor,'renderer':renderer,'version':version,'pixel':list(pixel),'hardware':True}),flush=True);success=True
 except Exception as exc:print('hardware context failed',repr(exc),flush=True)
 finally:
  current(display,None,None,None)
  if surf:destroy_surface(display,surf)
  if ctx:destroy_context(display,ctx)
  terminate(display)
raise SystemExit(0 if success else 2)
