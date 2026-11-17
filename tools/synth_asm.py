"""Assemble the published voice and its DSP dependencies without algorithm changes."""
import pathlib,re,struct
from x86_asm import assemble
ROOT=pathlib.Path(__file__).resolve().parents[1]
def voice_code(address=0x1000000,sections=()):
 source=(ROOT/'.reference/fr_public/werkkzeug3_kkrieger/_viruz2a.asm').read_text(encoding='cp1252')
 text=source[source.index('fastatan:'):source.index('; table for mod sources')]
 for start,end in sections:text+='\n'+source[source.index(start):source.index(end,source.index(start))]
 fields={}
 def structure(m):
  offset=0;name=m[1]
  for line in m[2].splitlines():
   line=line.split(';')[0].strip()
   if not line:continue
   f=re.fullmatch(r'\.(\w+)(?:\s+(resd|resb)\s+([\w.]+))?',line)
   assert f,(name,line)
   fields[name+'.'+f[1]]=offset
   if f[2]:offset+=(int(f[3]) if f[3].isdigit() else fields[f[3]])*(4 if f[2]=='resd' else 1)
  return ''
 text=re.sub(r'struc\s+(\w+)\s*\n(.*?)endstruc',structure,text,flags=re.S)
 lines=['.set icnoisemul,196314165','.set icnoiseadd,907633515','.set LOWEST,0x39000000']+[f'.set {name},{offset}' for name,offset in fields.items()];scope=''
 for line in text.splitlines():
  line=line.split(';')[0].strip()
  if not line or line.startswith(('section','global','V2Perf','%','_')):continue
  if re.fullmatch(r'sy\w+:?\s*',line):
   line=line.rstrip(':')+':'
  if line.endswith(':') and not line.startswith('.'):scope=line[:-1]
  line=re.sub(r'(?<![\w.])\.(\w+)',lambda m:scope+'.'+m[1],line)
  line=re.sub(r'\b([0-9][0-9a-fA-F]*)h\b',r'0x\1',line)
  line=re.sub(r'\b(dword|qword|word|byte)\s+\[',r'\1 ptr [',line)
  line=re.sub(r'\bbyte\s+(-?\d+)',r'\1',line)
  line=re.sub(r'\bst([0-7])\b',r'st(\1)',line).replace('cs:','')
  line=re.sub(r'\bshort\s+','',line)
  line=re.sub(r'^(mov\s+\w+,\s*)(vcebuf2?|chanbuf|aux[ab]buf)\b',r'\1offset \2',line)
  if re.search(r'\bdd\s',line):
   label,data=re.split(r'\bdd\s+',line,maxsplit=1);line=(label.strip()+': ' if label.strip() else '')+'.long '+data.rstrip(',')
  elif re.fullmatch(r'\w+\.\w+',line):line+=':'
  lines.append(line)
 for name,kind,value in re.findall(r'^((?:fc|SR|dcoffset)\w*)\s+(dd|dq)\s+([^;\r\n]+)',source,re.M):
  vals=[]
  for v in value.split(','):
   v=v.strip();bits=int.from_bytes(struct.pack('<d' if kind=='dq' else '<f',float(v)),'little') if '.' in v or 'e' in v.lower() else int(v)
   vals.append(str(bits))
  lines.append(name+': '+('.quad ' if kind=='dq' else '.long ')+','.join(vals))
 lines.append('temp: .zero 64')
 for name,n in [('vcebuf',1120),('vcebuf2',1120),('chanbuf',2240),('auxabuf',2240),('auxbbuf',2240),('aux2buf',1120)]:lines.append(f'{name}: .zero {n}')
 return (*assemble('\n'.join(lines),address),fields)
if __name__=='__main__':
 code,symbols,fields=voice_code();print(len(code),fields['syWV2.size'])
