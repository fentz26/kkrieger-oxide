#!/usr/bin/env python3
"""Original SetPlayer with no additive collision cells, as in menu scenes."""
import pathlib,subprocess,tempfile
import verify_checkpoints as reference
root=reference.root
driver=reference.driver[:reference.driver.index('int main()')].replace('g.Player.Collider.Cell->Id','g.Player.Collider.Cell?g.Player.Collider.Cell->Id:~0u')+r'''
int main(){word(64);for(int c=0;c<64;c++){KKriegerGame game;game.JumpTimer=7;game.SpeedUp=0.2f;game.OnGround=1;game.PlayerMat.Init();sVector p{float(c%7),c*0.125f,float(-(c%5)),float(c%2)};vector(p);game.SetPlayer(p,0.25f,-0.5f);dump(game);}}
'''
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'nullable.cpp';exe=pathlib.Path(tmp)/'nullable';cpp.write_text(reference.support+reference.functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/nullable-player.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 64 original nullable player placements')
