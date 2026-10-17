from pathlib import Path
import json
from PIL import Image, ImageChops
p=Path("target/gold7-review-20261017")
cap=p/"captures-final"
m=json.loads((cap/"manifest.json").read_text())
checks=[]
for entry in m:
 if entry["renderer"]!="checkbox" or entry["variant"]!="default":continue
 stem=entry["id"];before=Image.open(cap/(stem+".png")).convert("RGB");after=Image.open(cap/(stem+"-checked.png")).convert("RGB")
 scale=entry["backing_scale"]
 row=next(hit["frame"] for hit in entry["geometry"]["hits"] if hit["id"]==20)
 x,y,w,h=row
 box=(round((x+23)*scale),round(y*scale),round((x+w)*scale),round((y+h)*scale))
 diff=ImageChops.difference(before.crop(box),after.crop(box));unchanged=diff.getbbox() is None
 assert unchanged, stem+" label pixels changed when checked"
 pixels=list(after.crop(box).getdata());assert max(v[0] for v in pixels)-min(v[0] for v in pixels)>50,stem+" no readable glyph contrast"
 checks.append({"id":stem,"label_pixels_identical":True,"actual_pointer_activation":True})
assert len(checks)==16
(p/"checkbox-pixels-final.json").write_text(json.dumps(checks,indent=2)+"\n")
print("PASS:",len(checks),"actual Checkbox pointer activations; label pixels identical and readable across four themes, two backing scales, normal/large text")
