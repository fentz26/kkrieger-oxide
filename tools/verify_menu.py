#!/usr/bin/env python3
"""Original SetIf, State and Menu handlers, recording state during child execution."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
root=oracle.root;base=root/'.reference/fr_public/werkkzeug3_kkrieger';oracle.source=(base/'kdoc.cpp').read_text(encoding='cp1252')
functions='\n'.join(oracle.function(s) for s in ['void __stdcall Exec_Misc_SetIf(','void __stdcall Exec_Misc_State(','void __stdcall Exec_Misc_Menu('])
support=r'''
#include <cstdio>
#define __stdcall
#define sKEY_UP 0x10000
#define sKEY_DOWN 0x10001
#define sKEY_LEFT 0x10002
#define sKEY_RIGHT 0x10003
#define sKEY_ENTER 10
#define sKEY_MOUSEL 0x10028
#define sKEY_ESCAPE 27
#define sKEY_BACKSPACE 8
using sInt=int;
struct sVector{float x,y,z,w;void Init(float a,float b,float c,float d){x=a;y=b;z=c;w=d;}};
struct Game{unsigned char Switches[256];unsigned LastKey;};struct KEnvironment{struct Game*Game;sVector Var[32];};
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float f){fwrite(&f,4,1,stdout);}void vec(sVector v){real(v.x);real(v.y);real(v.z);real(v.w);}
void dump(KEnvironment*e){for(auto b:e->Game->Switches)word(b);word(e->Game->LastKey);for(int i=0;i<4;i++)vec(e->Var[i]);}
struct KOp{int Calls=0;void ExecInputs(KEnvironment*e){Calls++;dump(e);}};
template<class T>T sRange(T v,T hi,T lo){return v>=hi?hi:v<=lo?lo:v;}
'''
driver=r'''
int main(){word(256);unsigned keys[]={0,32,10,sKEY_MOUSEL,8,27,sKEY_UP,sKEY_DOWN,sKEY_LEFT,sKEY_RIGHT,0x80010000u,'w',sKEY_UP|0x40000000u};
 for(int c=0;c<256;c++){
  Game game;KEnvironment env{&game};KOp op;for(int i=0;i<256;i++)game.Switches[i]=(c+i)%7;game.LastKey=keys[c%13];for(int i=0;i<4;i++)env.Var[i]={float(i),float(i+1),float(i+2),float(i+3)};dump(&env);
  int sw=c%256,val=c%9-4,flags=c,osw=(c*7)%256;for(int v:{sw,val,flags,osw})word(v);
  word((flags&15)<=6);Exec_Misc_SetIf(&op,&env,sw,val,flags,osw);word(op.Calls);dump(&env);
  int state_sw=c%5==0?0:(c*5)%256,state_val=c-128,cond=c%10,condition_sw=c%8,condition_val=c%7;for(int v:{state_sw,state_val,cond,condition_sw,condition_val})word(v);
  Exec_Misc_State(&op,&env,state_sw,state_val,cond,condition_sw,condition_val);word(op.Calls);dump(&env);
  int menu_sw=(c*3)%256,max=c%11-1,menu_flags=c%4;for(int v:{menu_sw,max,menu_flags})word(v);
  Exec_Misc_Menu(&op,&env,menu_sw,max,menu_flags);word(op.Calls);dump(&env);
 }
}
'''
support='#include <initializer_list>\n'+support
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'menu.cpp';exe=pathlib.Path(tmp)/'menu';cpp.write_text(support+functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/menu.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 768 original menu/switch handler executions')
