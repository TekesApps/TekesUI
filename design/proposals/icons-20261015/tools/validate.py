#!/usr/bin/env python3
from pathlib import Path
import json, hashlib, xml.etree.ElementTree as ET
from collections import Counter
root=Path(__file__).resolve().parents[1]
m=json.loads((root/'manifest.json').read_text())
names=[i['name'] for i in m['items']]
assert len(names)==len(set(names))==51
assert Counter(i['group'] for i in m['items'])=={'shared':10,'cad':14,'drama':14,'refine':8,'rebuild':5}
assert Counter(i['kind'] for i in m['items'])=={'new':38,'refine':8,'rebuild':5}
allowed={'svg','path','rect','circle'}
hashes={}
for name in names:
 p=root/'svg'/f'{name}.svg';raw=p.read_bytes();svg=ET.fromstring(raw)
 assert svg.attrib['viewBox']=='0 0 24 24'
 assert svg.attrib['stroke-width']=='1.5'
 assert svg.attrib['stroke']=='currentColor'
 assert svg.attrib['stroke-linecap']==svg.attrib['stroke-linejoin']=='round'
 assert len(raw)<256*1024
 for node in svg.iter():
  assert node.tag.rsplit('}',1)[-1] in allowed
  assert not any(k.startswith('on') or 'href' in k for k in node.attrib)
  assert all(node.attrib.get(k) in (None,'none','currentColor') for k in ['fill','stroke'])
 assert all(word not in raw.lower() for word in [b'c2pa',b'<metadata',b'<script',b'<image',b'<text',b'<filter'])
 hashes[p.name]=hashlib.sha256(raw).hexdigest()
assert len(list((root/'svg').glob('*.svg')))==51
print(json.dumps({'svg_count':51,'groups':dict(Counter(i['group'] for i in m['items'])),'schema_style_external_references':'pass','sha256':hashes},ensure_ascii=False,indent=2))
