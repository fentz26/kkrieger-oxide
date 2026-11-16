#!/usr/bin/env python3
"""Original portal visibility with cached sectors absent from SectorJobs."""
import pathlib,subprocess,tempfile
import verify_visibility as reference
root=reference.root
driver=reference.driver.replace('word(128);for(int c=0;c<128;c++)','word(64);for(int c=0;c<64;c++)')
old='sectors[i].Next=engine.SectorJobs;engine.SectorJobs=&sectors[i];'
new='sectors[i].Next=nullptr;if((i+c)%3!=0){sectors[i].Next=engine.SectorJobs;engine.SectorJobs=&sectors[i];}'
assert old in driver;driver=driver.replace(old,new)
driver=driver.replace('matrix(engine.ViewProject);word(all?', 'word(c);matrix(engine.ViewProject);word(all?',1)
with tempfile.TemporaryDirectory() as tmp:
 cpp=pathlib.Path(tmp)/'cached.cpp';exe=pathlib.Path(tmp)/'cached';cpp.write_text(reference.support+reference.functions+driver)
 subprocess.run(['clang++','-std=c++17','-O2','-ffp-contract=off',str(cpp),'-o',str(exe)],check=True);data=subprocess.check_output([str(exe)])
out=root/'tests/fixtures/cached-sectors.bin';out.write_bytes(data);print(f'{out}: {len(data)} bytes, 64 original cached-sector visibility graphs')
