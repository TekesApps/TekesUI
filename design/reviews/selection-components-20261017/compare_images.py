from pathlib import Path
import json,hashlib,sys
from PIL import Image,ImageChops
p=Path('target/selection-components-20261017');a=p/sys.argv[1];b=p/sys.argv[2];out=p/sys.argv[3]
af={f.name:f for f in a.glob('*.png')};bf={f.name:f for f in b.glob('*.png')};assert af.keys()==bf.keys() and len(af)>0
cases=[]
for n in sorted(af):
 ah=hashlib.sha256(af[n].read_bytes()).hexdigest();bh=hashlib.sha256(bf[n].read_bytes()).hexdigest();bbox=None
 if ah!=bh:
  diff=ImageChops.difference(Image.open(af[n]).convert('RGBA'),Image.open(bf[n]).convert('RGBA'));bbox=diff.getbbox(alpha_only=False)
  if bbox:
   d=p/(sys.argv[3]+'-diff');d.mkdir(exist_ok=True);diff.save(d/n)
 cases.append(dict(file=n,before_sha256=ah,after_sha256=bh,pixel_identical=bbox is None,diff_bbox=bbox))
r=dict(captures=len(cases),pixel_identical=sum(c['pixel_identical'] for c in cases),cases=cases)
out.write_text(json.dumps(r,indent=2)+'\n');print(json.dumps({k:v for k,v in r.items() if k!='cases'}))
print('changed:',[c['file'] for c in cases if not c['pixel_identical']])
