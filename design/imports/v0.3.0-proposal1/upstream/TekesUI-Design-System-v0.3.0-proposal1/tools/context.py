#!/usr/bin/env python3
"""Generate local handoff views; never upload to Figma or certify native code.

The Figma output is a Tekes planning format, NOT an API request/import format.
All artifacts are deterministic projections; source edits require regeneration.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import sys
from validate import ROOT, THEMES, read, resolve

OUTPUTS = ('design/adapters/generated/agent-context.json',
           'design/adapters/generated/figma.variables-plan.json',
           'docs/AGENT-HANDOFF.md')


def encode(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False)+'\n').encode('utf-8')


def source_errors(adoption: dict | None = None) -> list[str]:
    sources=[]
    for path in ('research/claude-figma.sources.json','research/jenny-wen.sources.json'):
        sources.extend(read(path)['sources'])
    ids=[x['id'] for x in sources]
    errors=[]
    if len(ids)!=len(set(ids)): errors.append('DUPLICATE_CONTEXT_SOURCE_ID')
    for source in sources:
        if not source.get('url','').startswith('https://') or not source.get('access'):
            errors.append('INCOMPLETE_CONTEXT_SOURCE: '+source['id'])
    adoption=read('design/quality/reference-adoption.json') if adoption is None else adoption
    for row in adoption['decisions']:
        if not set(row['source_ids'])<=set(ids): errors.append('UNKNOWN_CONTEXT_SOURCE: '+row['id'])
        for target in row['artifacts']:
            path=(ROOT/target.split('#')[0]).resolve()
            if ROOT not in path.parents or not path.is_file():
                errors.append('MISSING_ADOPTION_ARTIFACT: '+target)
    contract=read('design/adapters/contract.json')
    if contract['appearance_modes']!=THEMES: errors.append('ADAPTER_THEME_CONTRACT')
    if contract['brand_is_component_variant']: errors.append('BRAND_IS_COMPONENT_VARIANT')
    if contract['figma_payload']!='tekes-mapping-plan-not-native-api': errors.append('ADAPTER_SCOPE_CLAIM')
    craft=read('design/quality/craft-review.json')
    if craft.get('release_required') is not True: errors.append('CRAFT_GATE_REQUIRED')
    check_ids=[row['id'] for row in craft['checks']]
    if len(check_ids)!=len(set(check_ids)) or len(check_ids)!=7: errors.append('CRAFT_CHECK_COVERAGE')
    for row in craft['checks']:
        if not set(row['source_ids'])<=set(ids): errors.append('UNKNOWN_CRAFT_SOURCE: '+row['id'])
        if row['status'] not in ('pass','fail','unverified'): errors.append('INVALID_CRAFT_STATUS: '+row['id'])
        if row['status']=='pass' and (not row.get('reviewer') or not row.get('evidence_uri')):
            errors.append('CRAFT_PASS_WITHOUT_EVIDENCE: '+row['id'])
    return errors


def expected_artifacts() -> dict[str,bytes]:
    errors=source_errors()
    if errors: raise ValueError('\n'.join(errors))
    gallery=read('design/brand/gallery.json')
    metadata=read('design/tokens/token-metadata.json')
    tokens=read('design/tokens/tokens.json')
    profiles=[]
    inputs=['SPEC.md','design/adapters/contract.json','design/brand/gallery.json',
            'design/tokens/base.tokens.json','design/tokens/token-metadata.json',
            'design/tokens/tokens.schema.json','design/brand/policy.json',
            'design/brand/brand.schema.json','design/component-contracts.json',
            'design/interaction-contracts.json','design/layout.json','design/quality/fixtures.json','design/registry/components.json',
            'design/registry/recipes.json','design/registry/pages.json',
            'design/quality/requirements-traceability.json',
            'design/quality/reference-adoption.json','design/quality/craft-review.json',
            'research/claude-figma.sources.json','research/jenny-wen.sources.json',
            'tools/context.py']
    for path in gallery['profiles']:
        config=read(path)
        receipt_path=f"design/brand/generated/{config['brand_id']}/receipt.json"
        receipt=read(receipt_path)
        profiles.append({'config_path':path,'config':config,'receipt_path':receipt_path,
                         'theme_contract_sha256':receipt['theme_contract_sha256']})
        inputs.extend((path,receipt_path))
    hashes={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in sorted(inputs)}
    context_hash=hashlib.sha256(json.dumps(hashes,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    common={'format_version':'1.0','design_version':metadata['profile_version'],
            'source_files_sha256':hashes,'context_sha256':context_hash,
            'status':'local-projection; online-roundtrip-and-native-runtime-unverified'}
    variables=[]
    for family,group in tokens.items():
        if family=='type':continue
        for token in group['tokens']:
            key=family+':'+token['name'];meta=metadata['tokens'][key]
            value=token['value'];alias=value[1:-1] if isinstance(value,str) and value.startswith('{') else None
            variables.append({'token_id':key,'proposed_display_name':family+'/'+token['name'],
                              'figma_variable_key':None,'unit':meta['unit'],'tier':meta['tier'],
                              'type':'COLOR' if family=='color' else 'FLOAT',
                              'alias_target':family+':'+alias if alias else None,
                              'alias_context':'active-brand-collection; roundtrip-unverified',
                              'reference_values':{t:resolve(tokens,family,token['name'],t) for t in THEMES},
                              'description':token.get('usage',''),'lifecycle':meta['status']})
    from brand import MANAGED
    extensions=[]
    for profile in profiles:
        bid=profile['config']['brand_id'];compiled=read(f'design/brand/generated/{bid}/tokens.json')
        extensions.append({'brand_id':bid,'brand_version':profile['config']['version'],
                           'figma_collection_key':None,'parent':'shared-structure',
                           'overrides':{key:{t:resolve(compiled,'color',key[6:],t) for t in THEMES}
                                        for key in sorted('color:'+name for name in MANAGED)},
                           'all_other_values':'inherit-from-shared-parent',
                           'theme_contract_sha256':profile['theme_contract_sha256']})
    figma={**common,'format':'tekes-figma-mapping-plan-not-native-api',
           'parent_reference_brand':gallery['reference_brand'],
           'reference_is_framework_default':False,'modes':THEMES,
           'variables':variables,'brand_extensions':extensions,
           'composite_typography':{'source':'design/tokens/tokens.json#type',
                                  'actual_style_bindings':None,'status':'unverified'},
           'unit_conversion':'logical-pt preserved; native tool conversion unverified'}
    components=[]
    for component in read('design/registry/components.json')['components']:
        components.append({**component,'source_path':None,'code_commit':None,'figma_component_key':None,
                           'binding_status':'unverified; API is proposed, not a compiled symbol'})
    agent={**common,'format':'tekes-agent-design-context',
           'authority':read('design/adapters/contract.json'),
           'brand_profiles':profiles,'token_catalog':variables,
           'typography_contract':tokens['type'],'layout_contract':read('design/layout.json'),
           'fixtures':read('design/quality/fixtures.json'),
           'component_contracts':read('design/component-contracts.json'),
           'components':components,'recipes':read('design/registry/recipes.json')['recipes'],
           'pages':read('design/registry/pages.json')['pages'],
           'interaction_contracts':read('design/interaction-contracts.json'),
           'craft_review':read('design/quality/craft-review.json'),
           'not_permission_to_change':['business behavior','authorization','IME','undo history','shared design authority'],
           'required_before_import':['real source revision','approved brand','native component and page evidence']}
    handoff=f'''# Agent 设计交接索引\n\n版本：`{metadata['profile_version']}`。上下文摘要：`{context_hash}`。\n\n由 `tools/context.py` 生成；所有 API 是提案，不能当作已存在的 Rust 符号。\n\n[主规范](../SPEC.md) · [品牌契约](BRANDING.md) · [组件/API上下文](../design/adapters/generated/agent-context.json) · [Figma映射规划](../design/adapters/generated/figma.variables-plan.json)\n\n显式参考品牌：{', '.join(p['config']['brand_id'] for p in profiles)}，每品牌四主题。品牌不是组件variant，未配置不得默认为陶土。\n\n40项组件、10项消费层recipe和真实页面契约来自登记表。使用语义token；保留别名图，不以解析色值反写设计源。界面自动修复只能提出明确迁移差异，歧义留待评审。\n\nFigma变量键、组件键、Rust源码位置/commit均未核实，保持null。规划不是在线导入证明，颜色表不是原生Gallery。\n\n[Claude/Figma参考](REFERENCE-POSTS.md) · [Jenny Wen原帖与访谈](JENNY-WEN-POSTS.md) · [真实体验评审](../design/quality/craft-review.json)\n\n流程可以缩短，验收不能省略。L1组件、L2应用、L3真实环境各自留证；批准版本与生成物绑定，修改后重验。\n'''
    return dict(zip(OUTPUTS,(encode(agent),encode(figma),handoff.encode('utf-8'))))


def freshness_errors(artifacts: dict[str,bytes]) -> list[str]:
    errors=['STALE_CONTEXT_OUTPUT: '+p for p,data in artifacts.items()
            if not (ROOT/p).is_file() or (ROOT/p).read_bytes()!=data]
    actual={str(p.relative_to(ROOT)) for p in (ROOT/'design/adapters/generated').rglob('*') if p.is_file()}
    required={p for p in artifacts if p.startswith('design/adapters/generated/')}
    if actual!=required: errors.append('ORPHAN_CONTEXT_OUTPUT')
    return errors


def write_artifacts(artifacts: dict[str,bytes]) -> None:
    for relative,data in artifacts.items():
        p=ROOT/relative;p.parent.mkdir(parents=True,exist_ok=True)
        temporary=p.with_suffix(p.suffix+'.tmp');temporary.write_bytes(data);temporary.replace(p)


def validate_all(write_report: bool = False) -> tuple[list[str],dict]:
    artifacts=expected_artifacts();errors=freshness_errors(artifacts)
    report={'scope':'local-context-and-reference-traceability','status':'fail' if errors else 'pass',
            'errors':errors,'generated_views':list(artifacts),
            'online_roundtrip':'unverified','native_component_binding':'unverified'}
    if write_report:(ROOT/'reports/context-validation.json').write_bytes(encode(report))
    return errors,report


def main() -> int:
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    artifacts=expected_artifacts()
    if args.check:
        errors=freshness_errors(artifacts)
        print('\n'.join(errors) if errors else 'Local design-context views are current; online roundtrips unverified.')
        return int(bool(errors))
    write_artifacts(artifacts);print('Generated local Agent context and Figma mapping plan; no external workspace changed.')
    return 0


if __name__=='__main__':
    try:sys.exit(main())
    except (OSError,ValueError,KeyError,TypeError) as error:
        print('CONTEXT_ERROR: '+str(error),file=sys.stderr);sys.exit(2)
