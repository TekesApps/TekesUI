from pathlib import Path
import json,math
from PIL import Image,ImageChops,ImageDraw
p=Path('target/gold7-review-20261017');v=json.load(open(p/'captures-final/manifest.json'));assert len(v)==743 and len({c['component'] for c in v})==45
assert not [(c['id'],c['geometry']['outside_preview_hit_ids']) for c in v if c['geometry']['outside_preview_hit_ids']]
chart=[(c,b) for c in v for b in c['geometry']['chart'] or []]
assert chart and all(b['value_ink'] and b['category_ink'] for _,b in chart)
assert all(b['value_center_error_pt']<=1.0 and b['category_center_error_pt']<=1.0 for _,b in chart)
icons=[]
for c in v:
 if c['renderer']!='icon-button':continue
 scale=c['backing_scale'];hit=next(h['frame'] for h in c['geometry']['hits'] if h['id']==20);cx=(hit[0]+hit[2]/2)*scale;cy=(hit[1]+hit[3]/2)*scale;im=Image.open(p/'captures-final'/(c['id']+'.png')).convert('RGB');n=12*scale;crop=im.crop((round(cx-n),round(cy-n),round(cx+n),round(cy+n)));dark='-dark' in c['id'];mask=crop.convert('L').point(lambda x:255 if (x>128 if dark else x<128) else 0)
 hd=ImageChops.difference(mask,mask.transpose(Image.Transpose.FLIP_LEFT_RIGHT)).getbbox();vd=ImageChops.difference(mask,mask.transpose(Image.Transpose.FLIP_TOP_BOTTOM)).getbbox();icons.append({'id':c['id'],'horizontal_symmetric':hd is None,'vertical_symmetric':vd is None,'ink_bbox_in_crop':mask.getbbox(),'button_center_pixel':[cx,cy]});assert hd is None and vd is None,c['id']
closed=next(c for c in v if c['id']=='component-disclosure-large-narrow-light-2x');opened=next(c for c in v if c['id']==closed['id']+'-open');r=next(h['frame'] for h in closed['geometry']['hits'] if h['id']==20);scale=2;box=(round((r[0]+36)*scale),round((r[1]+2)*scale),round((r[0]+r[2]-12)*scale),round((r[1]+r[3]-2)*scale));a=Image.open(p/'captures-final'/(closed['id']+'.png')).crop(box);b=Image.open(p/'captures-final'/(opened['id']+'.png')).crop(box);raw_same=ImageChops.difference(a,b).getbbox(alpha_only=False) is None
# Real pointer input changes the semantic hover/background color. Compare fully covered
# white glyph pixels, whose positions are independent of antialias compositing.
ma=a.convert('L').point(lambda x:255 if x>250 else 0);mb=b.convert('L').point(lambda x:255 if x>250 else 0)
same=ma.getbbox() is not None and ImageChops.difference(ma,mb).getbbox() is None;assert same,'Disclosure fully covered label ink shifted'
r={'component_count':45,'rendered_cases':len(v),'default_cases':720,'actual_input_open_cases':7,'actual_input_checked_cases':16,'out_of_preview_hit_rects':0,'chart_labels':len(chart)*2,'chart_max_value_ink_center_error_pt':max(b['value_center_error_pt'] for _,b in chart),'chart_max_category_ink_center_error_pt':max(b['category_center_error_pt'] for _,b in chart),'iconbutton_symmetry':icons,'disclosure_label_closed_open_core_ink_identical':same,'disclosure_label_closed_open_raw_rgb_identical':raw_same,'note':'Center error measures visible glyph ink, not just slot frames. Antialiased glyph bearings/pixel rounding permit up to 1 logical point; Disclosure raw RGB differs because actual pointer input changes the background; fully covered label ink stays identical. No physical display or timing claim.'}
(p/'pixel-checks.json').write_text(json.dumps(r,indent=2)+'\n')
# Readable review sheets; these are cropped actual rendered pages, not stand-in component drawings.
large=[c for c in v if c['id'].endswith('-large-narrow-light-1x')]
for k in range(3):
 sheet=Image.new('RGB',(1710,1700),'#ddd');d=ImageDraw.Draw(sheet)
 for i,c in enumerate(large[k*15:(k+1)*15]):
  x=(i%3)*570;y=(i//3)*340;pr=c['geometry']['preview'];im=Image.open(p/'captures-final'/(c['id']+'.png')).convert('RGB');region=im.crop((int(pr[0])-2,int(pr[1])-2,int(pr[0]+pr[2])+2,int(pr[1]+min(pr[3],400))+2));region.thumbnail((566,310));sheet.paste(region,(x+2,y+26));d.text((x+10,y+7),c['component'],fill='#111')
 sheet.save(p/f'components-large-{k+1}.png')
print(json.dumps({k:r[k] for k in ['component_count','rendered_cases','out_of_preview_hit_rects','chart_max_value_ink_center_error_pt','chart_max_category_ink_center_error_pt','disclosure_label_closed_open_core_ink_identical']}))
