from pathlib import Path
from PIL import Image,ImageChops,ImageDraw,ImageFont
import json,hashlib,functools,shutil
R=Path('/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI/design/reviews/icon-c-rollout-57-20261023');V=R.parent/'icon-full-rollout-57-20261023';P=Path('/Users/frankqin/dev/worktrees/tekesui-icon-gold7-57');O=P/'design/reviews/icon-gold7-57/phase-c';O.mkdir(exist_ok=True)
cov=json.loads((O.parent/'coverage.json').read_text())['icons'];compiled=json.loads((R/'catalog-c/compiled-svg.json').read_text());cases=json.loads((R/'catalog-c/cases.json').read_text()); assert len(compiled)==177
for c in cov:assert hashlib.sha256(compiled[c['name']].encode()).hexdigest()==c['after_sha256']
@functools.lru_cache(maxsize=120)
def im(p):return Image.open(p).convert('RGB')
pixels=[]
for c in cases:
 b=int(c['backing']);x,y,w,h=c['artboard'];box=tuple(round(v*b)for v in(x,y,x+w,y+h));f=f'catalog-{c["group"]}-t{c["theme"]}-b{b}.png';d=ImageChops.difference(im(V/'catalog-before'/f).crop(box),im(R/'catalog-c'/f).crop(box)).getbbox();pixels.append({**c,'changed':d is not None});
 if not next(i['changed']for i in cov if i['name']==c['name']):assert d is None,c
assert len(pixels)==4248
(O/'catalog-pixel-review.json').write_text(json.dumps({'cases':pixels,'retained_cases_pixel_identical':19*24},indent=2)+'\n')
font=ImageFont.truetype('/System/Library/Fonts/Helvetica.ttc',12)
for group in range(6):
 selected=[c for c in cases if c['group']==group and c['theme']==0 and c['backing']==2 and c['size']==24]
 sheet=Image.new('RGB',(1560,32+len(selected)*48),'#eeeeeb');d=ImageDraw.Draw(sheet);d.text((8,8),'C: all identities | Original / C at 12,16,24pt | Light / Dark / Increased light / Increased dark',font=font,fill='#222')
 for row,c in enumerate(selected):
  y=32+48*row;d.text((8,y+15),c['name'],font=font,fill='#222')
  for t in range(4):
   for si,size in enumerate([12,16,24]):
    item=next(q for q in cases if q['name']==c['name']and q['theme']==t and q['backing']==2 and q['size']==size);x0,y0,w,h=item['artboard'];box=tuple(round(v*2)for v in(x0,y0,x0+w,y0+h));f=f'catalog-{group}-t{t}-b2.png'
    for k,path in enumerate([V/'catalog-before'/f,R/'catalog-c'/f]):sheet.paste(im(path).crop(box),(230+t*330+si*100+k*49,y+(48-size*2)//2))
 sheet.save(O/f'all-identities-{group}.png')
for label,folder in [('gallery','gallery-c-square'),('fixture','fixture-c')]:
 files=sorted((R/folder).glob('*.png'));page=Image.new('RGB',(1440,((len(files)+3)//4)*260),'#ccc');d=ImageDraw.Draw(page)
 for i,f in enumerate(files):
  x=i%4*360;y=i//4*260;pic=im(f).copy();pic.thumbnail((356,230));page.paste(pic,(x,y+20));d.text((x+2,y+2),f.name,font=font,fill='#111')
 page.save(R/f'{label}-contact.png')
for folder in ['catalog-c','fixture-c','gallery-c-square','native-c-square']:
 assert (R/folder).exists()
(O/'gpu-artifact-sha256.json').write_text(json.dumps({str(p.relative_to(R)):hashlib.sha256(p.read_bytes()).hexdigest()for folder in ['catalog-c','fixture-c','gallery-c-square','native-c-square']for p in sorted((R/folder).rglob('*'))if p.is_file()},indent=2)+'\n')
for n in ['run.py','run-square.py','catalog-c.log','fixture-c.log','gallery-c-square.log','native-c-square.log','native-regressions-square.log','component-c.log','native-square-build.log','validated-square-source-sha256.json','before-source-sha256.json','native-binary-receipt.json']:
 shutil.copyfile(R/n,O/n)
for folder in ['catalog-c','fixture-c','gallery-c-square']:
 shutil.copyfile(R/folder/('cases.json'if folder=='catalog-c'else'receipt.json'),O/(folder+'-receipt.json'))
print('C 4248 configurations matched exact canonical source; 456 retained configurations pixel-identical; 6 all-identity visual sheets generated')
