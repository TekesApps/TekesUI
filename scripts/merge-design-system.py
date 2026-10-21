#!/usr/bin/env python3
"""Compile proposal base + consumer brand + retained legacy API into one Rust bundle.

Run with Python 3.11+ and the imported proposal's pinned requirements. This is an
explicit build-time step; Rust UI/runtime never imports or launches Python.
--check is read-only, never rebaselines screenshots or approves a design release.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import importlib
import json
from pathlib import Path
import struct
import sys

# The upstream archive is immutable, including absence of Python cache artifacts.
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = Path('design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1')
THEMES = ['light','dark','light-ic','dark-ic']
SOURCE_FILES = ['design/source/base.tokens.json','design/source/legacy.tokens.json',
                'design/source/rust-mapping.json','design/source/motion.json','design/source/token-metadata.json',
                'design/source/layout.json','design/source/interaction-contracts.json',
                'design/brand/config.json','design/brand/policy.json','design/brand/gallery.json',
                'scripts/merge-design-system.py','scripts/generate-gallery-brands.py',
                'design/gallery/brands.json']
UPSTREAM_INPUTS = ['tools/brand.py','tools/brand_color.py','tools/validate.py',
                   'design/tokens/token-metadata.json','design/tokens/tokens.schema.json',
                   'design/brand/brand.schema.json','design/brand/policy.schema.json',
                   'design/quality/contrast-pairs.json']


def strict_pairs(items):
    result={}
    for key,value in items:
        if key in result: raise ValueError('duplicate JSON key: '+key)
        result[key]=value
    return result


def load(path):
    return json.loads(path.read_text(),object_pairs_hook=strict_pairs,
                      parse_constant=lambda x:(_ for _ in ()).throw(ValueError('nonfinite '+x)))


def bytes_json(value):
    return (json.dumps(value,ensure_ascii=False,indent=2,sort_keys=True,allow_nan=False)+'\n').encode()


def semantic_bytes(v):
    """Shared with Rust: numbers are IEEE f64, avoiding language JSON formatting drift."""
    if v is None: return b'n'
    if isinstance(v,bool): return b't' if v else b'f'
    if isinstance(v,(int,float)): return b'd'+struct.pack('>d',v)
    if isinstance(v,str):
        b=v.encode();return b's'+len(b).to_bytes(8,'big')+b
    if isinstance(v,list):return b'a'+len(v).to_bytes(8,'big')+b''.join(map(semantic_bytes,v))
    return b'o'+len(v).to_bytes(8,'big')+b''.join(semantic_bytes(k)+semantic_bytes(v[k]) for k in sorted(v))


def f32(x):return struct.unpack('>f',struct.pack('>f',x))[0]


def token_value(bundle,family,name,theme='light',stack=()):
    key=(family,name)
    if key in stack:raise ValueError('cyclic proposal alias: '+str(stack+(key,)))
    found=[t for t in bundle[family]['tokens'] if t['name']==name]
    if len(found)!=1:raise ValueError('unknown/duplicate proposal token: '+family+':'+name)
    value=found[0]['value']
    if isinstance(value,str) and value.startswith('{') and value.endswith('}'):
        return token_value(bundle,family,value[1:-1],theme,stack+(key,))
    return value[theme] if isinstance(value,dict) else value


def pointer(d,path):
    for key in path.strip('/').split('/'):d=d[key]
    return d


def merge(root=ROOT,config_path=None):
    sys.path.insert(0,str(root/UPSTREAM/'tools'))
    brand=importlib.import_module('brand')
    config=load(config_path or root/'design/brand/config.json')
    base=load(root/'design/source/base.tokens.json');policy=load(root/'design/brand/policy.json')
    bundle,palette=brand.compile_config(config,base=base,policy=policy,
                                      metadata=load(root/'design/source/token-metadata.json'))
    result=load(root/'design/source/legacy.tokens.json');mapping=load(root/'design/source/rust-mapping.json')
    motion=load(root/'design/source/motion.json')
    styles={style['name']:style for group in bundle['type']['groups'] for style in group['styles']}
    changes=[]
    for binding in mapping['bindings']:
        leaf=pointer(result,binding['legacy']);kind=binding['kind'];name=binding['target']
        if leaf['$value']!={'binding':True}:raise ValueError('mapped value must not be authored twice: '+binding['legacy'])
        if kind=='color':
            value=token_value(bundle,'color',name,binding['theme'])
            rgb=[f32(int(value[i:i+2],16)/255) for i in (1,3,5)]
            alpha=1.0
            rule=mapping['render_opacity'].get(name)
            if rule:alpha=f32(token_value(bundle,'opacity',rule['increased' if binding['theme'].endswith('-ic') else 'default']))
            value={'colorSpace':'srgb','components':rgb,'alpha':alpha}
        elif kind=='opacity':value=f32(token_value(bundle,'opacity',name))
        elif kind=='metric':value={'value':f32(token_value(bundle,binding['family'],name)),'unit':'pt'}
        elif kind=='motion':value={'value':f32(motion['duration'][name]),'unit':'s'}
        elif kind=='text':
            value=styles[name][binding['field']]
            value=int(value) if binding['field']=='fontWeight' else {'value':f32(value),'unit':'pt'}
        else:raise ValueError('unknown mapping kind '+kind)
        leaf['$value']=value
        changes.append({**binding,'resolved_value':value})
    gallery=load(root/'design/brand/gallery.json')
    # A config is explicit; default file is copied from the package reference, never blue by accident.
    result['proposal']={'tokens':bundle,'render_opacity':mapping['render_opacity'],'motion':motion,'layout':load(root/'design/source/layout.json'),
                        'interaction':load(root/'design/source/interaction-contracts.json'),
                        'brand':{'config':config,'reference_brand':gallery['reference_brand'],'gallery':gallery,'policy':policy,'palette':palette}}
    inputs={str(path):hashlib.sha256((root/path).read_bytes()).hexdigest()
            for path in [*map(Path,SOURCE_FILES),*(UPSTREAM/Path(p) for p in UPSTREAM_INPUTS)]}
    if config_path is not None and config_path.resolve()!=(root/'design/brand/config.json').resolve():
        raise ValueError('Use --root with a copied source tree for isolated consumer builds; config authority is design/brand/config.json')
    projection_sha=hashlib.sha256(semantic_bytes(result)).hexdigest()
    result['merge']={'format':'tekesui.projection/1','inputs_sha256':inputs,'projection_sha256':projection_sha,
                     'generator':'scripts/merge-design-system.py','upstream_archive_sha256':'cafbe021196d2ec9f575df45a3b0433892d0e2cc3cce36b60937227e1a0ccb9c'}
    return result,{'format':'tekesui.proposal-mapping/1','brand':config['brand_id'],
                   'bindings':changes,'unmatched_text_roles':mapping['unmatched_text_roles'],
                   'new_proposal_counts':{k:len(v.get('tokens',[])) for k,v in bundle.items() if k!='type'},
                   'text_profiles':list(styles),'native_evidence':'unverified; requires new Gallery render review'}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path,default=ROOT)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args();root=args.root.resolve()
    document,report=merge(root)
    outputs={root/'design/tokens.json':bytes_json(document),root/'design/proposal-mapping.json':bytes_json(report)}
    if args.check:
        stale=[str(p.relative_to(root)) for p,data in outputs.items() if not p.is_file() or p.read_bytes()!=data]
        if stale:raise ValueError('STALE_MERGED_OUTPUT: '+', '.join(stale))
        print('Proposal + consumer brand + legacy merge is current; no native or release approval implied.')
    else:
        for path,data in outputs.items():
            tmp=path.with_suffix(path.suffix+'.tmp');tmp.write_bytes(data);tmp.replace(path)
        print('Generated design/tokens.json and proposal-mapping.json; native review still required.')

if __name__=='__main__':
    try:main()
    except (OSError,ValueError,KeyError,TypeError,ImportError) as error:
        print('DESIGN_MERGE_ERROR: '+str(error),file=sys.stderr);sys.exit(1)
