#!/usr/bin/env python3
"""Capture pinned published Windows GDI font/atlas routines on the current fonts.

Font routines, packing and UV loops are unchanged source extractions. GdiFlush
is an observation barrier before copying the DIB; original batching timing is
not reproduced. No font files or captured font pixels are redistributed.
"""
import argparse, hashlib, os, pathlib, subprocess, tempfile, urllib.request
ROOT = pathlib.Path(__file__).resolve().parent.parent
COMMIT = '8c8f82c2ae01133370e649bbc05916e7aceecadd'
HASHES = ['4736567785c1f9388b82b5a7f27ec89b439cc5289e11b19129ac4f9895fa6377', '5393370804cdbd9ef647779d79987b95b1f0e3cfa06f10e6b6b38a3b2579f7ef', '38a7db093edce02775819d23c0276a782a4eabfb23545ef19faff61bc7b2800e', '57888180230e4e767d9d8a95aa923cef618118f8931a4fa13efe078cb5271235', '090d2bf0d8c90d157ee961a46125e956baa68f60e220ec6855a3692fe6fe77f0', 'ce86226450c000e578340d62c4ba27794a41641cb6706fc5cfe7c3e82907d289', '334a3187175826e20c923b35fd962393f6b447100a469d29dd3ba7decd25da4a']

def extract(source, needle):
    start = source.index(needle)
    end = source.index('{', start) + 1
    depth = 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return source[start:end]

def source(name):
    path = ROOT / '.reference/fr_public/werkkzeug3_kkrieger' / name
    if path.exists(): data = path.read_bytes()
    else:
        url = f'https://raw.githubusercontent.com/farbrausch/fr_public/{COMMIT}/werkkzeug3_kkrieger/{name}'
        with urllib.request.urlopen(url, timeout=60) as response: data = response.read()
    return data.decode('cp1252').replace('\r\n', '\n')

def generate():
    system, bitmap = source('_start.cpp'), source('genbitmap.cpp')
    methods = [extract(system, n) for n in [
        'sInt sSystem_::FontBegin(', 'void sSystem_::FontCharWidth(',
        'void sSystem_::FontPrint(sInt x,sInt y,const sChar *', 'void sSystem_::FontEnd(']]
    parts = methods + [extract(bitmap, 'struct letterz'),
        extract(bitmap, 'if(page)                            // font mode'),
        extract(bitmap[bitmap.index('sSystem->FontEnd();'):], 'if(page)')]
    assert [hashlib.sha256(s.encode()).hexdigest() for s in parts] == HASHES
    prefix = r"""
#undef UNICODE
#undef _UNICODE
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <initializer_list>
#include <io.h>
#include <fcntl.h>
using sInt=int;using sU32=uint32_t;using sU16=uint16_t;
using sU8=uint8_t;using sS16=int16_t;using sChar=char;
#define sSetMem std::memset
#define sDPrintF(...) ((void)0)
int sAlign(int value,int align){return (value+align-1)&~(align-1);}
HDC GDIScreenDC,GDIDC;HBITMAP GDIHBM,FontHBM;HFONT FontHandle;
BITMAPINFO FontBMI{};
struct Rect{float x0,y0,x1,y1;};
struct Metric{Rect UV;float PreSpace,Width;};
using KLetterMetric=Metric;
struct Env{Metric Letters[8][256];};
struct sSystem_ {
 uint32_t*FontMem=nullptr;
 int FontBegin(int,int,const char*,int,int,int);
 void FontCharWidth(int,int*);
 void FontPrint(int,int,const char*,int);
 void FontEnd();
};
void word(uint32_t v){fwrite(&v,4,1,stdout);}
void real(float v){fwrite(&v,4,1,stdout);}
"""
    body = prefix + '\n'.join(methods) + '\n' + parts[4] + ';\n'
    body += r"""
int main(){
 _setmode(_fileno(stdout),_O_BINARY);
 GDIScreenDC=GetDC(nullptr);GDIDC=CreateCompatibleDC(GDIScreenDC);
 GDIHBM=CreateCompatibleBitmap(GDIScreenDC,16,16);SelectObject(GDIDC,GDIHBM);
 if(!GDIScreenDC||!GDIDC||!GDIHBM)return 1;
 word(24);
 for(int test=0;test<24;test++){
  int fontId=test%2;
  const char*font=fontId?"Times New Roman":"Arial";
  float x=(test%3-1)*0.03125f,y=((test/3)%2-1)*0.03125f;
  float width=(2+(test/6)%4)*0.0625f,height=((test/2)%2)*0.03125f;
  int extspace=(test/4)%3-1;float inspace=((test/5)%3-1)/512.0f;
  word(fontId);real(x);real(y);real(width);real(height);word(extspace);real(inspace);
  int xs=512,ys=256,alias=4,page=1;
  int es=extspace*alias+2*alias,is=int(inspace*xs);
  int xp=int(x*xs)+es,yp=int(y*ys)+es;
  int count,chr,widths[3],w0,w1,w2,i;
  letterz let[256]{};Env env{};Env*kenv=&env;sSystem_ system;sSystem_*sSystem=&system;
  HGDIOBJ oldScreenFont=GetCurrentObject(GDIScreenDC,OBJ_FONT);
  HGDIOBJ oldMemoryFont=GetCurrentObject(GDIDC,OBJ_FONT);
  int yf=sSystem->FontBegin(xs,ys,font,int(xs*height),int(ys*width),4);
  if(yf<=0||!system.FontMem)return 2;
  char content[]=" -~";char*text=content;
""" + parts[5] + '\n' + parts[6] + r"""
  // Complete native drawing before observing shared DIB memory.
  if(!GdiFlush())return 3;
  word(yf);
  for(int c=0;c<256;c++){
   word(let[c].xp);word(let[c].yp);word(let[c].w0);word(let[c].w1);word(let[c].w2);
   auto&m=kenv->Letters[page][c];
   for(float f:{m.UV.x0,m.UV.y0,m.UV.x1,m.UV.y1,m.PreSpace,m.Width})real(f);
  }
  for(int row=0;row<64;row++)for(int col=0;col<128;col++){
   unsigned sum=0;
   for(int yy=0;yy<4;yy++)for(int xx=0;xx<4;xx++)
    sum+=system.FontMem[(row*4+yy)*xs+col*4+xx]&255;
   unsigned char coverage=sum>>4;fwrite(&coverage,1,1,stdout);
  }
  sSystem->FontEnd();
  SelectObject(GDIScreenDC,oldScreenFont);SelectObject(GDIDC,oldMemoryFont);
  DeleteObject(FontHandle);
 }
 DeleteDC(GDIDC);DeleteObject(GDIHBM);ReleaseDC(nullptr,GDIScreenDC);
 return ferror(stdout)?4:0;
}
"""
    return body

def main():
    p=argparse.ArgumentParser();p.add_argument('--emit-source',type=pathlib.Path)
    p.add_argument('--output',type=pathlib.Path,default=ROOT/'.reference/gdi-font.bin')
    args=p.parse_args();code=generate()
    if args.emit_source:
        args.emit_source.write_text(code);return
    if os.name!='nt':raise SystemExit('native GDI capture requires Windows')
    vswhere=pathlib.Path(os.environ['ProgramFiles(x86)'])/'Microsoft Visual Studio/Installer/vswhere.exe'
    install=subprocess.check_output([str(vswhere),'-latest','-products','*','-requires',
        'Microsoft.VisualStudio.Component.VC.Tools.x86.x64','-property','installationPath'],text=True).strip()
    if not install:raise SystemExit('MSVC C++ tools are required')
    vcvars=pathlib.Path(install)/'VC/Auxiliary/Build/vcvars64.bat'
    with tempfile.TemporaryDirectory() as directory:
        directory=pathlib.Path(directory);cpp=directory/'reference.cpp';exe=directory/'reference.exe'
        cpp.write_text(code)
        command=subprocess.list2cmdline(['cl','/nologo','/std:c++17','/EHsc','/O2','/fp:strict',
            str(cpp),'/Fe:'+str(exe),'/link','gdi32.lib','user32.lib'])
        subprocess.run(f'cmd.exe /d /s /v:off /c "call "{vcvars}" >nul && {command}"',cwd=directory,check=True)
        data=subprocess.check_output([str(exe)])
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_bytes(data)
    print(f'{len(data)} bytes, SHA-256 {hashlib.sha256(data).hexdigest()}')
if __name__=='__main__':main()
