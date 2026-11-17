#!/usr/bin/env python3
"""Original Ronan lyric sequencing, synchronization, controllers and DSP callbacks."""
from speech_reference import ROOT,run,source
texts=['!DHAX_ !kwIH_k !br4AH_UHn !fAA_ks !jAH_mps !OW!vE_R !lEY!zIY_ !dAA_g','sil ng th sh dh zh ch ih eh ae ah oh uh ax iy er aa ao uw','2ah3iy 12ow !ax_','', '?ABC xyz !ng_','j dh zh v z ','0002aa 0aw 3oy','!aa_ !ee_ !iy_']
# Literal ASCII spellings are serialized as input as well as embedded in the C++ driver.
import json,struct,re
block=re.search(r'}\s*syls\[\]\s*=\s*\{(.*?)\};',source,re.S)[1]
texts += re.findall(r'\{\"([^\"]*)\",\{',block)
assert len(texts)==60
literals=','.join(json.dumps(t) for t in texts)
driver=r'''
void word(unsigned n){fwrite(&n,4,1,stdout);}void real(float n){fwrite(&n,4,1,stdout);}void frame(const Ronan::syVRonan&f){fwrite(&f,4,25,stdout);}
void state(){auto&w=Ronan::workspace;real(Ronan::pitch);word(Ronan::framerate);word(w.framecount);word(w.spos);word(w.scounter);word(w.cursyl);word(w.durfactor);real(w.invdur);word(w.ptr-w.baseptr);word(w.curp1);word(w.curp2);word(w.wait4on);word(w.wait4off);frame(w);frame(w.newframe);for(auto&r:w.res){real(r.p1);real(r.p2);}real(w.lastin2);word(w.nseed);real(w.nout);real(w.hpb1);real(w.hpb2);real(w.peq1.p1);real(w.peq1.p2);}
int main(){const char*input[]={TEXTS};word(64);word(64);unsigned seed=1234;for(unsigned c=0;c<64;c++){unsigned rates[]={22050,44100,48000,96000};unsigned rate=rates[c%4];word(rate);Ronan::pitch=0;memset(Ronan::texts,0,sizeof(Ronan::texts));ronanCBSetSR(rate);ronanCBInit();const char*lyrics[64]={};for(int i=0;i<60;i++)lyrics[i]=input[i];synthSetLyrics(lyrics);state();for(unsigned step=0;step<64;step++){unsigned action=0,value=0;if(step==0){action=3;value=c%60;}else if(step==1){action=3;value=64+c%4;}else if(step==2){action=4;value=c*13%129;}else if(step==31)action=5;else if(step%23==0){action=3;value=(c+step)%61;}else if(step%13==0)action=2;else if(step%11==0)action=1;word(action);word(value);switch(action){case 1:ronanCBNoteOn();break;case 2:ronanCBNoteOff();break;case 3:ronanCBSetCtl(4,value);break;case 4:ronanCBSetCtl(5,value);break;case 5:ronanCBInit();break;}ronanCBTick();unsigned n=0;if(step%4==3){unsigned counts[]={1,7,64,128};n=counts[(c+step/4)%4];}word(n);float data[256];for(unsigned i=0;i<n*2;i++){seed=seed*1664525+1013904223;data[i]=((int)((seed>>16)%1024)-512)/512.f;}fwrite(data,4,n*2,stdout);if(n)ronanCBProcess(data,n);fwrite(data,4,n*2,stdout);state();}}}
'''.replace('TEXTS',literals)
result=struct.pack('<I',len(texts))+b''.join(struct.pack('<I',len(t))+t.encode() for t in texts)+run(driver)
path=ROOT/'tests/fixtures/synth-speech-sequence.bin';path.write_bytes(result);print(f'{len(result)} bytes; 4096 original lyric/control/render ticks')
