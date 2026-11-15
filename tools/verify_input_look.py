#!/usr/bin/env python3
"""Unchanged original OnTick mouse-look prefix and Euler conversion."""
import pathlib,subprocess,tempfile
import verify_mesh as oracle
import verify_player_motion as motion
root=oracle.root;source=(root/'.reference/fr_public/werkkzeug3_kkrieger/kkriegergame.cpp').read_text(encoding='cp1252')
a=source.index('  time = sSystem->GetTime()-LastTime;');b=source.index('\n\n  // weapon animation',a)
function='\nvoid KKriegerGame::LookPrefix(){int time;float f;sInputData id;\n'+source[a:b]+'\n}\n'
support=motion.support
support=support.replace('struct KKriegerGame {','''struct sInputData{int Analog[2];};struct System{int Time,Mouse[2];int GetTime(){return Time;}void GetInput(int,sInputData&id){id.Analog[0]=Mouse[0];id.Analog[1]=Mouse[1];}}systemObject;System*sSystem=&systemObject;
struct KKriegerGame {int LastTime,LastMouseX,LastMouseY,FlyMode;unsigned char Switches[256]{};float PlayerLook,MouseTurnSpeed,MouseLookSpeed;void LookPrefix();
''')
support+='\nconstexpr float sPIF=3.14159265358979323846f;\n#define KGS_MOUSESPEED 13\n#define KGS_MOUSEINVERT 12\ntemplate<class T>T sRange(T v,T hi,T lo){return v>=hi?hi:v<=lo?lo:v;}\n'
driver=r'''
int main(){word(128);word(8);for(int c=0;c<128;c++){
 KKriegerGame game;game.LastTime=c*100;game.LastMouseX=c*13;game.LastMouseY=-c*11;game.PlayerDir=(c%9-4)*0.125f;game.PlayerLook=(c%7-3)*0.25f;game.MouseTurnSpeed=0.0005f*(1+c%5);game.MouseLookSpeed=0.00075f*(1+c%3);
 word(game.LastTime);word(game.LastMouseX);word(game.LastMouseY);for(float f:{game.PlayerDir,game.PlayerLook,game.MouseTurnSpeed,game.MouseLookSpeed})real(f);
 for(int k=0;k<8;k++){
  int delta[]={0,999,1000,1001,-7,20,5000,10};sSystem->Time=game.LastTime+delta[k];sSystem->Mouse[0]=c*13+k*31-70;sSystem->Mouse[1]=-c*11+k*47-100;game.FlyMode=(c+k)%2;game.Switches[13]=(c+k)%10;game.Switches[12]=c%3;
  word(sSystem->Time);word(sSystem->Mouse[0]);word(sSystem->Mouse[1]);word(game.FlyMode);word(game.Switches[13]);word(game.Switches[12]);
  game.LookPrefix();word(game.LastTime);word(game.LastMouseX);word(game.LastMouseY);real(game.PlayerDir);real(game.PlayerLook);for(auto v:{game.PlayerMat.i,game.PlayerMat.j,game.PlayerMat.k,game.PlayerMat.l})vector(v);
 }
}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'look.cpp';exe=pathlib.Path(tmp)/'look';cpp.write_text(support+motion.functions+function+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/input-look.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 1024 original mouse-look updates')
