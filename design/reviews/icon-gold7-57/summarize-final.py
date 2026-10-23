from pathlib import Path
from PIL import Image,ImageChops,ImageDraw,ImageFont
import json,hashlib,functools,shutil,subprocess
R=Path(__file__).resolve().parent;PUBLIC=Path('/Users/frankqin/dev/worktrees/tekesui-icon-gold7-57');OUT=PUBLIC/'design/reviews/icon-gold7-57'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
cov=json.loads((OUT/'coverage.json').read_text())['icons'];changed={c['name'] for c in cov if c['changed']}
compiled=json.loads((R/'catalog-final/compiled-svg.json').read_text())
assert len(compiled)==len(cov)==177
for c in cov:assert hashlib.sha256(compiled[c['name']].encode()).hexdigest()==c['after_sha256']
cases=json.loads((R/'catalog-final/cases.json').read_text())
@functools.lru_cache(maxsize=100)
def img(folder,name):return Image.open(R/folder/name).convert('RGB')
pixels=[]
for c in cases:
    b=int(c['backing']);x,y,w,h=c['artboard'];box=tuple(round(v*b)for v in(x,y,x+w,y+h));file=f'catalog-{c["group"]}-t{c["theme"]}-b{b}.png'
    diff=ImageChops.difference(img('catalog-before',file).crop(box),img('catalog-final',file).crop(box)).getbbox()
    if c['name']not in changed:assert diff is None,(c['name'],c['size'],b)
    pixels.append({**c,'changed':diff is not None})
assert len(pixels)==4248 and {x['name']for x in pixels if x['changed']}==changed
native=[]
for i,name in enumerate(['check','chevron-right','chevron-down','close','plus','search','folder','file','info','spinner']):
    files=list((R/'native-before').glob(f'native-{i}-*.png'));assert len(files)==24
    diffs=sum(ImageChops.difference(Image.open(f).convert('RGB'),Image.open(R/'native-proposed-2'/f.name).convert('RGB')).getbbox()is not None for f in files)
    native.append({'identity':name,'configurations':24,'changed_configurations':diffs,'rule':'retain existing rounded spinner bars'if i==9 else'original primitive backend, rounded strokes/bends; original semantic identity'})
font=ImageFont.truetype('/System/Library/Fonts/Helvetica.ttc',13);chosen=[c['name']for c in cov if c['changed']];byname={c['name']:c for c in cases if c['theme']==0 and c['backing']==2 and c['size']==24}
sheet=Image.new('RGB',(1360,((len(chosen)+3)//4)*104+34),'#f4f2ef');d=ImageDraw.Draw(sheet);d.text((14,10),'FINAL: Original / Soft contour | actual 24pt @2x GPU crops',font=font,fill='#222')
for k,n in enumerate(chosen):
    c=byname[n];x=k%4*340;y=34+k//4*104;d.text((x+8,y+4),n,font=font,fill='#222');row=c['row']
    for j,folder in enumerate(['catalog-before','catalog-final']):sheet.paste(img(folder,f'catalog-{c["group"]}-t0-b2.png').crop((1020,2*(42+row*36),1100,2*(74+row*36))),(x+30+j*130,y+28))
sheet.save(R/'all-final-changed.png')
gallery=json.loads((R/'gallery-final-2/receipt.json').read_text());assert gallery['frames']==144
fixture=json.loads((R/'fixture-final-after/receipt.json').read_text());assert len(fixture)==64
artifacts={str(p.relative_to(R)):sha(p)for folder in ['catalog-before','catalog-final','gallery-before','gallery-final-2','fixture-final-before','fixture-final-after','native-before','native-proposed-2']for p in sorted((R/folder).rglob('*'))if p.is_file()}
(OUT/'gpu-artifact-sha256.json').write_text(json.dumps(artifacts,indent=2)+'\n')
(OUT/'native-path-coverage.json').write_text(json.dumps(native,indent=2)+'\n')
(OUT/'catalog-pixel-review.json').write_text(json.dumps({'cases':pixels,'retained_cases_pixel_identical':129*24,'changed_names':sorted(changed)},indent=2)+'\n')
for name in ['all-final-changed.png','native-focused.png','native-settings-light.jpg','native-settings-dark.jpg','native-settings-light-ic.jpg','native-settings-dark-ic.jpg','native-settings-light-ic-150.jpg','native-settings-narrow-150.jpg','sdk-input-sha256.json','sdk-supplement-sha256.json','final-source-sha256.json','component-final.log','native-regression.log','native-regression-2.log','native-proposed-2.log','gallery-final-2.log','run-final.py','summarize-final.py','whole-page-test-origin.json','native-gpu-test.rs']:
    p=R/name
    if p.is_file():shutil.copyfile(p,OUT/name)
for folder,file in [('catalog-final','catalog-0-t0-b1.png'),('catalog-final','catalog-0-t3-b2.png'),('gallery-final-2','0-brand1-1440x960-interior@2x.png'),('gallery-final-2','3-brand2-720x480-interior@1x.png'),('fixture-final-after','page-t0-b1-brand1-nfalse-s1.png')]:
    shutil.copyfile(R/folder/file,OUT/(folder+'-'+file))
(OUT/'summary.json').write_text(json.dumps({'issue':57,'public_base_revision':'50ec49f3446924fdbc66b3cff22cd8cfe150fe7c','canonical':{'total':177,'changed':48,'retained':129,'square_canvas':[24,24],'GPU_configurations_per_phase':4248,'compiled_artwork_exact':True},'native_primitive_backend':native,'Gallery_whole_page_pairs':144,'shared_component_fixture_pairs':64,'native_window_observed':['four modes','settings 12/16/24pt','150% typography','actual narrow window and scrolling','text/focus survived mode and window changes'],'system_input':'CUA native input; not physical keyboard/mouse','physical_IME':'not tested; no editing or composition code changed','VoiceOver':'not tested; no semantic or AX-owner code changed','performance':'out of scope; not measured','raw_local_evidence':str(R),'release_pin':'publish exact commit after PR merge; local overrides are explicit development inputs'},indent=2)+'\n')
print('Final 177 identities, 48/129 coverage, 4248 matched catalog configurations, 144 Gallery pairs, 64 fixture pairs; source/artifact identities bound')
