#!/usr/bin/env python3
"""Generate brand tokens, documentation and expectations, never native screenshots."""
from __future__ import annotations
import itertools
from validate import ROOT, read, dump, resolve, component_document, recipe_document, THEMES


def profile_receipts():
    profiles=[]
    for path in read('design/brand/gallery.json')['profiles']:
        config=read(path)
        receipt=read(f"design/brand/generated/{config['brand_id']}/receipt.json")
        profiles.append({'brand_id':config['brand_id'],'brand_version':config['version'],
                         'theme_contract_sha256':receipt['theme_contract_sha256']})
    return profiles


def case_payloads():
    registry=read('design/registry/components.json'); profiles=profile_receipts()
    cases=[]
    for brand in profiles:
        for c in registry['components']:
            for theme,state in itertools.product(THEMES,c['states']+c['state_combinations']):
                cases.append({**brand,'component':c['id'],'theme':theme,'state':state,'fixture':'short','required_evidence':'native-runtime','status':'unverified'})
            for theme,fixture in itertools.product(THEMES,c['fixtures']):
                if fixture!='short':
                    cases.append({**brand,'component':c['id'],'theme':theme,'state':'rest','fixture':fixture,'required_evidence':'native-runtime','status':'unverified'})
    recipes=read('design/registry/recipes.json')['recipes'];recipe_cases=[]
    for brand in profiles:
        for recipe in recipes:
            for theme,state,fixture in itertools.product(THEMES,recipe['states'],recipe['fixtures']):
                recipe_cases.append({**brand,'recipe':recipe['id'],'theme':theme,'state':state,'fixture':fixture,'status':'unverified','required_evidence':'consumer-native-runtime','optional_recipe':recipe['optional']})
    pages=[]
    for brand in profiles:
        for page in read('design/registry/pages.json')['pages']:
            for theme in THEMES:
                pages.append({**brand,'page':page['id'],'theme':theme,'status':'unverified','required_evidence':'real-consumer-page','note':'plus applicable page states and scales from page contract'})
    return {
        'reports/expected-gallery-cases.json':{'scope':'minimum explicit brand/theme/state cases; native runner must add scale and risk combinations','cases':cases},
        'reports/expected-recipe-cases.json':{'scope':'consumer recipes across explicit brands; optional recipe gate separate','cases':recipe_cases},
        'reports/expected-page-brand-cases.json':{'scope':'minimum real-page brand/theme coverage, not runtime evidence','cases':pages},
    }


def palette_document(tokens,brand_id):
    rows=['# 四主题语义色表', '', f'显式 Gallery 参考品牌：`{brand_id}`。本表不代表系统固定品牌；消费者必须编译自己的配置。',
          '由 `tools/generate.py` 生成。数值通过不等于最终像素通过。', '',
          '| Token | Light | Dark | Light-IC | Dark-IC |','|---|---|---|---|---|']
    for token in tokens['color']['tokens']:
        if not token['name'].startswith('p-'):
            rows.append('| '+token['name']+' | '+' | '.join(resolve(tokens,'color',token['name'],theme) for theme in THEMES)+' |')
    return '\n'.join(rows)+'\n'


def brand_comparison():
    rows=['# 品牌派生对照（生成结果，不是 Gallery）','',
          '只比较同一套语义角色如何随消费者基色改变。Neutral/Blue/Clay 是测试夹具，不是系统身份。',
          '表中全部数值由配置与算法生成；真实布局、状态输入、辅助技术和像素仍未验证。','']
    for path in read('design/brand/gallery.json')['profiles']:
        c=read(path);p=read(f"design/brand/generated/{c['brand_id']}/palette.json")
        rows += [f"## {c['brand_id']} · {c['seed_srgb']}",'',
                 '| Role | Light | Dark | Light-IC | Dark-IC |','|---|---|---|---|---|']
        for role in p['roles']['light']:
            rows.append('| '+role+' | '+' | '.join(p['roles'][t][role] for t in THEMES)+' |')
        rows += ['']
    return '\n'.join(rows)+'\n'


def documentation_errors():
    tokens=read('design/tokens/tokens.json');bid=read('design/brand/gallery.json')['reference_brand']
    expected={'docs/PALETTE.md':palette_document(tokens,bid),'docs/BRAND-EXAMPLES.md':brand_comparison()}
    return ['STALE_GENERATED_DOCUMENT: '+p for p,s in expected.items() if not (ROOT/p).exists() or (ROOT/p).read_text(encoding='utf-8')!=s]


def main():
    from brand import expected_artifacts,write_artifacts
    artifacts,_=expected_artifacts();write_artifacts(artifacts)
    registry=read('design/registry/components.json')
    for component in registry['components']:
        p=ROOT/component['spec'];p.parent.mkdir(parents=True,exist_ok=True)
        p.write_text(component_document(component),encoding='utf-8')
    for recipe in read('design/registry/recipes.json')['recipes']:
        p=ROOT/recipe['spec'];p.parent.mkdir(parents=True,exist_ok=True)
        p.write_text(recipe_document(recipe),encoding='utf-8')
    for path,data in case_payloads().items():dump(path,data)
    tokens=read('design/tokens/tokens.json')
    dump('reports/resolved-colors.json',{theme:{t['name']:resolve(tokens,'color',t['name'],theme) for t in tokens['color']['tokens'] if not t['name'].startswith('p-')} for theme in THEMES})
    bid=read('design/brand/gallery.json')['reference_brand']
    (ROOT/'docs/PALETTE.md').write_text(palette_document(tokens,bid),encoding='utf-8')
    (ROOT/'docs/BRAND-EXAMPLES.md').write_text(brand_comparison(),encoding='utf-8')
    from context import expected_artifacts as context_artifacts, write_artifacts as write_context
    write_context(context_artifacts())
    print('Generated local handoff views and three brand profiles × four themes, 40 component docs, 10 recipe docs and native expectations. Runtime remains unverified.')


if __name__=='__main__':main()
