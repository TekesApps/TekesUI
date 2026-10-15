#!/usr/bin/env python3
"""Repeatable positive, adversarial and edge-seed tests for the brand compiler.
These are static tests. They do not operate a native Gallery or an input method.
"""
from __future__ import annotations
import copy
import json
import random
import sys
from brand import (compile_config, config_errors, policy_errors, dependency_names,
                   freshness_errors, bytes_json, ROLE_NAMES)
from brand_color import contrast, parse_hex, to_oklab, to_oklch, tone
from validate import ROOT, THEMES, read, resolve


def run_tests() -> list[dict]:
    results=[]
    config=read('design/brand/examples/example-blue.json')
    def check(name,fn):
        try:
            assert fn(), name
            results.append({'name':name,'status':'pass','scope':'brand-static'})
        except Exception as error:
            results.append({'name':name,'status':'fail','scope':'brand-static','detail':str(error)})
    def rejects(name,mutate,prefix):
        def test():
            c=copy.deepcopy(config);mutate(c)
            try: compile_config(c)
            except ValueError as error:return prefix in str(error)
            return False
        check(name,test)
    def schema_reject(name,mutate):
        def test():
            c=copy.deepcopy(config);mutate(c);return bool(config_errors(c))
        check(name,test)
    check('sRGB black/white contrast equals 21',lambda:contrast('#000000','#FFFFFF')==21.)
    check('Oklab red conversion reference',lambda:all(abs(a-b)<1e-6 for a,b in zip(to_oklab('#FF0000'),(.6279553606,.2248630611,.1258462985))))
    check('quantized Oklab round trip',lambda:all(tone(*to_oklch(x))==x for x in ['#2563EB','#D97757','#808080','#00FF00','#FFFFFF','#000000']))
    blue,bp=compile_config(config)
    clay_config=read('design/brand/examples/example-clay.json');clay,cp=compile_config(clay_config)
    check('same seed/policy generates identical bytes',lambda:bytes_json(compile_config(config)[0])==bytes_json(blue))
    check('all four themes have all generated roles',lambda:all(set(bp['roles'][theme])==set(ROLE_NAMES) for theme in THEMES))
    check('brand decoration preserves exact consumer seed',lambda:all(resolve(blue,'color','brand-decoration',theme)==config['seed_srgb'] for theme in THEMES))
    check('blue vs clay changes all hue-bearing generated roles',lambda:all(bp['roles'][theme][r]!=cp['roles'][theme][r] for theme in THEMES for r in ROLE_NAMES if r not in ['action-primary-foreground','action-primary-focus-outline']))
    check('nonbrand roles are independent across blue/clay',lambda:all(resolve(blue,'color',x['name'],theme)==resolve(clay,'color',x['name'],theme) for x in blue['color']['tokens'] if x['name'] not in dependency_names(blue) for theme in THEMES))
    check('all non-color families unchanged across brands',lambda:all(blue[k]==clay[k] for k in blue if k!='color'))
    check('legacy clay name is an explicit seed-following alias',lambda:all(resolve(blue,'color','brand-clay',t)==config['seed_srgb'] for t in THEMES))
    schema_reject('reject missing seed',lambda c:c.pop('seed_srgb'))
    schema_reject('reject alpha-bearing seed',lambda c:c.update(seed_srgb='#2563EB80'))
    schema_reject('reject invalid hex seed',lambda c:c.update(seed_srgb='#GGGGGG'))
    schema_reject('reject numeric seed',lambda c:c.update(seed_srgb=123))
    schema_reject('reject named color seed',lambda c:c.update(seed_srgb='blue'))
    schema_reject('reject path traversal brand id',lambda c:c.update(brand_id='../brand'))
    schema_reject('reject unknown consumer field',lambda c:c.update(status_color='#2563EB'))
    schema_reject('reject incomplete brand version',lambda c:c.update(version='1.0'))
    schema_reject('reject unknown override role',lambda c:c['overrides'].update({'status-failed':{'reason':'must not change semantics','values':{t:'#FF0000' for t in THEMES}}}))
    schema_reject('reject incomplete four-theme override',lambda c:c['overrides'].update({'accent':{'reason':'missing theme is unsafe','values':{'light':'#000000'}}}))
    schema_reject('reject undocumented override',lambda c:c['overrides'].update({'accent':{'values':{t:'#FFFFFF' for t in THEMES}}}))
    rejects('reject blank override justification',lambda c:c['overrides'].update({'accent':{'reason':'         ','values':{t:bp['roles'][t]['accent'] for t in THEMES}}}),'BRAND_OVERRIDE_REASON')
    rejects('reject low-contrast override after generation',lambda c:c['overrides'].update({'accent':{'reason':'intentionally unreadable foreground','values':{t:'#FFFFFF' for t in THEMES}}}),'BRAND_CONTRAST')
    rejects('reject collapsed primary interaction states',lambda c:c['overrides'].update({'action-primary-hover':{'reason':'intentionally duplicates rest fill','values':{t:bp['roles'][t]['action-primary-ground'] for t in THEMES}}}),'BRAND_STATE_COLLAPSE')
    rejects('reject non-neutral on-solid foreground',lambda c:c['overrides'].update({'action-primary-foreground':{'reason':'intentionally tinted on-solid text','values':{t:('#000001' if t.startswith('dark') else '#FFFFFE') for t in THEMES}}}),'BRAND_ON_SOLID_NONNEUTRAL')
    override_config=copy.deepcopy(config)
    override_config['overrides']['accent']={'reason':'explicitly reviewed equivalent foreground for demonstration','values':{t:bp['roles'][t]['accent'] for t in THEMES}}
    check('valid explicit four-theme override is accepted',lambda:compile_config(override_config)[0]==blue)
    def bad_base(kind):
        base=read('design/tokens/base.tokens.json')
        if kind=='literal':base['color']['tokens'].append({'name':'p-brand-seed','value':{t:'#2563EB' for t in THEMES},'usage':'invalid'})
        elif kind=='leak':next(x for x in base['color']['tokens'] if x['name']=='status-failed')['value']='{accent}'
        else:next(x for x in base['color']['tokens'] if x['name']=='p-surface-2')['value']['light-ic']='#000000'
        try:compile_config(config,base=base)
        except ValueError as error:
            return {'literal':'BRAND_LITERAL_IN_BASE','leak':'BRAND_INDEPENDENT_ROLE_LEAK','infeasible':'BRAND_NO_FEASIBLE_COLOR'}[kind] in str(error)
        return False
    check('reject hardcoded brand primitive in base',lambda:bad_base('literal'))
    check('reject status accidentally rebound to brand',lambda:bad_base('leak'))
    check('reject infeasible mixed-surface IC contract',lambda:bad_base('infeasible'))
    def weak_policy():
        p=read('design/brand/policy.json');p['thresholds']['text']=1.;return bool(policy_errors(p))
    check('reject weakened contrast policy',weak_policy)
    check('reject stale generated token bytes',lambda:any(x.startswith('STALE_BRAND_OUTPUT') for x in freshness_errors({'design/tokens/tokens.json':b'wrong'})))
    seeds=['#FFFFFF','#000000','#FFFF00','#00FF00','#FF0000','#0000FF','#00FFFF','#FF00FF','#EEEEFF','#808080','#FFFDF0','#040406']
    for seed in seeds:
        def edge(seed=seed):
            c=copy.deepcopy(config);c['seed_srgb']=seed
            tokens,palette=compile_config(c)
            return all(resolve(tokens,'color','brand-decoration',t)==seed for t in THEMES)
        check('edge seed '+seed+' compiles all four themes with full contrast checks',edge)
    rng=random.Random(3001)
    for i in range(12):
        seed='#'+''.join(f'{rng.randrange(256):02X}' for _ in range(3))
        def sampled(seed=seed):
            c=copy.deepcopy(config);c['seed_srgb']=seed;compile_config(c);return True
        check('fixed-sample seed '+seed+' compiles all four themes',sampled)
    return results


if __name__=='__main__':
    result=run_tests()
    print(json.dumps(result,ensure_ascii=False,indent=2))
    (ROOT/'reports').mkdir(exist_ok=True)
    (ROOT/'reports/brand-tests.json').write_bytes(bytes_json(result))
    sys.exit(0 if all(x['status']=='pass' for x in result) else 1)
