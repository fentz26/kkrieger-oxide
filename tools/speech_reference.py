"""Compile the original Ronan callbacks with their original math declarations."""
import pathlib,subprocess,tempfile
ROOT=pathlib.Path(__file__).resolve().parents[1]
source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2.cpp').read_text(encoding='cp1252')
body=source[source.index('#pragma pack (push, 1)'):source.index('#define GETDELTA')]
support='''#include <cstdio>
extern "C" {double exp(double);double pow(double,double);double cos(double);double atan(double);double fabs(double);long lrintf(float);}
#include <cstring>
#define RONAN
#define __stdcall
using sU8=unsigned char;using sS8=signed char;using sS32=int;using sU32=unsigned;using sInt=int;using sF32=float;
double sFExp(double x){return exp(x);}double sFPow(double a,double b){return pow(a,b);}int sFtol(float x){return (int)lrintf(x);}
'''
def run(driver):
 with tempfile.TemporaryDirectory() as tmp:
  tmp=pathlib.Path(tmp);cpp=tmp/'speech.cpp';exe=tmp/'speech';cpp.write_text(support+body+driver)
  # _types.hpp has only double math declarations; avoid modern float overloads.
  subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off','-fno-builtin','-Wno-register','-Wno-parentheses',str(cpp),'-o',str(exe)],check=True)
  return subprocess.check_output([str(exe)])
