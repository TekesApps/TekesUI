#!/usr/bin/env python3
"""Validate this design proposal. Does not test Rust rendering, IME or accessibility."""
from __future__ import annotations
import argparse, copy, hashlib, json, math, pathlib, re, sys, tomllib
try:
    import jsonschema
except ImportError:
    raise SystemExit('Missing dependency: install jsonschema, then rerun.')
ROOT = pathlib.Path(__file__).resolve().parents[1]
THEMES = ['light', 'dark', 'light-ic', 'dark-ic']
BASE_STATES = {'rest','hover','pressed','focus','disabled','selected','checked','loading','error'}
def _object_pairs(pairs):
    out={}
    for key,value in pairs:
        if key in out: raise ValueError('DUPLICATE_JSON_KEY: '+key)
        out[key]=value
    return out

def _invalid_constant(value):
    raise ValueError('NONFINITE_JSON_NUMBER: '+value)

def read(path: str):
    return json.loads((ROOT/path).read_text(encoding='utf-8'),
                      object_pairs_hook=_object_pairs, parse_constant=_invalid_constant)
def dump(path: str, data):
    p=ROOT/path;p.parent.mkdir(parents=True,exist_ok=True)
    p.write_text(json.dumps(data,ensure_ascii=False,indent=2,allow_nan=False)+'\n',encoding='utf-8')
def resolve(tokens: dict, family: str, name: str, theme: str='light', stack: tuple=()):
    key=(family,name)
    if key in stack:
        raise ValueError(f'ALIAS_CYCLE: {stack + (key,)}')
    records=[t for t in tokens[family]['tokens'] if t['name']==name]
    if len(records)!=1:
        raise ValueError(f'ALIAS_TARGET: {family}:{name} resolves to {len(records)} tokens')
    value=records[0]['value']
    if isinstance(value,str) and value.startswith('{'):
        if not re.fullmatch(r'\{[a-z][a-z0-9_-]*\}',value):
            raise ValueError(f'ALIAS_SYNTAX: {value}')
        return resolve(tokens,family,value[1:-1],theme,stack+(key,))
    return value[theme] if isinstance(value,dict) else value

def rgb(color: str):
    return tuple(int(color[i:i+2],16)/255 for i in (1,3,5))
def luminance(c):
    v=[x/12.92 if x<=0.04045 else ((x+0.055)/1.055)**2.4 for x in c]
    return .2126*v[0]+.7152*v[1]+.0722*v[2]
def contrast(a,b):
    x,y=sorted([luminance(a),luminance(b)])
    return (y+.05)/(x+.05)
def contrast_results(tokens: dict):
    result=[]
    for pair in read('design/quality/contrast-pairs.json')['pairs']:
        for theme in THEMES:
            fg=rgb(resolve(tokens,'color',pair['foreground'],theme))
            bg=rgb(resolve(tokens,'color',pair['background'],theme))
            overlay=pair.get('overlay')
            if overlay:
                oc=rgb(resolve(tokens,'color',overlay['color'],theme))
                alpha=overlay.get('opacity_by_theme',{}).get(theme)
                if alpha is None:alpha=resolve(tokens,'opacity',overlay['opacity'],theme)
                # Rendering contract: source-over in sRGB for these solid UI recipes.
                # The native renderer must separately prove its final pixels match this assumption.
                bg=tuple(alpha*c+(1-alpha)*b for c,b in zip(oc,bg))
            ratio=contrast(fg,bg);minimum=pair['ic' if theme.endswith('-ic') else 'normal']
            result.append({'theme':theme,'foreground':pair['foreground'],'background':pair['background'],'overlay':overlay,'ratio':ratio,'minimum':minimum,'status':'pass' if ratio>=minimum else 'fail'})
    return result

def token_errors(tokens: dict, metadata: dict):
    errors=[]
    validator=jsonschema.Draft202012Validator(read('design/tokens/tokens.schema.json'))
    errors += [f'SCHEMA: {list(e.path)}: {e.message}' for e in validator.iter_errors(tokens)]
    if errors:return errors
    expected=set()
    for family,obj in tokens.items():
        if family=='type':continue
        seen=set()
        for token in obj['tokens']:
            n=token['name'];key=family+':'+n;expected.add(key)
            if n in seen:errors.append('DUPLICATE_TOKEN: '+key)
            seen.add(n)
            meta=metadata['tokens'].get(key)
            if not meta:
                errors.append('MISSING_METADATA: '+key);continue
            if meta['unit']!=metadata['family_units'][family]:errors.append('UNIT: '+key)
            tier=meta['tier'];value=token['value']
            is_alias=isinstance(value,str) and value.startswith('{')
            if tier not in ['primitive','semantic','component']:errors.append('TIER: '+key)
            if tier!='primitive' and not is_alias:errors.append('UNBOUND_LITERAL: '+key)
            if is_alias:
                target=metadata['tokens'].get(family+':'+value[1:-1])
                if not target:errors.append('MISSING_ALIAS_TARGET: '+key)
                elif {'primitive':0,'semantic':1,'component':2}[target['tier']]> {'primitive':0,'semantic':1,'component':2}[tier]:errors.append('REVERSE_DEPENDENCY: '+key)
            for theme in THEMES:
                try:resolved=resolve(tokens,family,n,theme)
                except (ValueError,KeyError) as e:errors.append(str(e));continue
                if family!='color' and (isinstance(resolved,bool) or not isinstance(resolved,(int,float))):errors.append('TOKEN_TYPE: '+key)
    if set(metadata['tokens'])!=expected:errors.append('ORPHAN_METADATA')
    fonts={f['name'] for f in tokens['type']['fonts']}
    families={f['name'] for f in tokens['type']['families']}
    for family in tokens['type']['families']:
        if not set(family['fonts'])<=fonts:errors.append('FONT_REFERENCE: '+family['name'])
    names=set()
    for group in tokens['type']['groups']:
        for style in group['styles']:
            if style['name'] in names:errors.append('DUPLICATE_TEXT_STYLE: '+style['name'])
            names.add(style['name'])
            if style['family'] not in families:errors.append('FONT_FAMILY: '+style['name'])
            if style['lineHeight']<style['fontSize']:errors.append('LINE_HEIGHT: '+style['name'])
            try:resolve(tokens,'color',style['color'][1:-1])
            except (ValueError,KeyError) as e:errors.append(str(e))
    return errors

def component_document(c: dict) -> str:
    states=', '.join(c['states']);combos=', '.join(c['state_combinations']) or '无附加组合；只读状态按登记展示。'
    na='\n'.join(f'| {s} | {r} |' for s,r in sorted(c['not_applicable'].items()))
    return f'''# {c['name']}

稳定 ID：`{c['id']}` · 分类：`{c['category']}` · crate：`{c['crate']}`

状态：**planned；没有运行时实现证据**。负责人角色：`{c['owner_role']}`；实际维护者须在进入 beta 前绑定。{'此 crate 为建议新增，需架构评审。' if c['crate_is_proposed_addition'] else ''}

## API 契约

以下为设计级签名，不是已编译的 Rust API：

```text
{c['api']}
```

使用：{c['usage']}。

禁区：{c['prohibited']}。

共有品牌派生契约见 `../BRANDING.md`；组件不得读取 seed 或固定色相名。共有契约见 `../../SPEC.md` 第 5、9、10、12 节及 `../ALIGNMENT.md`。该组件已登记的具体 token 和行为映射见 `../../design/component-contracts.json`。可视样式不能改变业务回调次数、payload、权限范围或撤销边界；组件只收受控值并发出语义事件。任意 style 参数只能接收类型化 token / variant，不能接收随意颜色或尺寸。

## 状态与主题

适用状态：{states}。

必测组合：{combos}。

四主题：light、dark、light-ic、dark-ic；分别使用 Gallery 登记的中性、蓝色、陶土色三品牌夹具。外观/品牌切换只更新同一完整 ThemeContext，不改变焦点、组字、草稿和选区。状态通过同一个 reducer 与正常输入事件建立，Gallery 不得另写造假外观。

| 不适用状态 | 理由 |
|---|---|
{na}

## Gallery 契约

拟定稳定地址：`{c['gallery_route']}`。目前不是可打开的页面。

夹具：{', '.join(c['fixtures'])}。页内固定顺序：规范、真实交互实例、状态矩阵、四主题并排、文字/窗口缩放、token 依赖、无障碍语义、验证记录。复制/选择原文不包含装饰信息。状态矩阵与无障碍树快照都由本登记生成。

## 验收与迁移

组件 / 实际应用 / 实机三层当前均为 `unverified`。不得把该文档或图片作为 Rust 实现证据。发布后 ID 与 slug 保持不变；废弃保留说明和替代组件，至少跨两个 minor 且不少于 90 天后才可在 major 版删除。
'''

def registry_errors(registry: dict, check_docs: bool=True):
    errors=[]
    errors += [f'REGISTRY_SCHEMA: {list(e.path)}: {e.message}' for e in jsonschema.Draft202012Validator(read('design/schemas/registry.schema.json')).iter_errors(registry)]
    if errors:return errors
    fixtures={f['id'] for f in read('design/quality/fixtures.json')['fixtures']}
    ids=set();slugs=set()
    for c in registry['components']:
        if c['id'] in ids:errors.append('DUPLICATE_COMPONENT: '+c['id'])
        if c['gallery_slug'] in slugs:errors.append('DUPLICATE_SLUG: '+c['gallery_slug'])
        ids.add(c['id']);slugs.add(c['gallery_slug'])
        if set(c['themes'])!=set(THEMES):errors.append('MISSING_COMPONENT_THEME: '+c['id'])
        states=set(c['states']);na=set(c['not_applicable'])
        if BASE_STATES-(states|na):errors.append('UNDECLARED_STATE: '+c['id'])
        if states&na:errors.append('CONTRADICTORY_STATE: '+c['id'])
        if len(c['states'])!=len(states):errors.append('DUPLICATE_STATE: '+c['id'])
        for combination in c['state_combinations']:
            if not set(combination.split('+'))<=states:errors.append('INVALID_COMBINATION: '+c['id']+':'+combination)
        if not set(c['fixtures'])<=fixtures:errors.append('UNKNOWN_FIXTURE: '+c['id'])
        if c['status'] not in ['planned','experimental','beta','stable','deprecated']:errors.append('INVALID_STATUS: '+c['id'])
        if c['status'] in ['beta','stable'] and not (c['owner_handle'] and c['code_symbol'] and c['gallery_implementation']):errors.append('FALSE_READY_COMPONENT: '+c['id'])
        if check_docs:
            p=ROOT/c['spec']
            if not p.exists() or p.read_text(encoding='utf-8')!=component_document(c):errors.append('STALE_COMPONENT_DOCUMENT: '+c['id'])
    expected={str(ROOT/c['spec']) for c in registry['components']}
    actual={str(p) for p in (ROOT/'docs/components').glob('*.md')}
    if check_docs and actual!=expected:errors.append('ORPHAN_OR_MISSING_COMPONENT_DOCUMENT')
    for page in read('design/registry/pages.json')['pages']:
        if not set(page['component_ids'])<=ids:errors.append('UNKNOWN_PAGE_COMPONENT: '+page['id'])
        if set(page['themes'])!=set(THEMES):errors.append('MISSING_PAGE_THEME: '+page['id'])
    return errors

def sidecar_errors(layout: dict, motion: dict, tokens: dict):
    errors=[]
    def nonnegative(value):
        return not isinstance(value,bool) and isinstance(value,(int,float)) and math.isfinite(value) and value>=0
    def walk(value,path):
        if isinstance(value,dict):
            for k,v in value.items():walk(v,path+'.'+k)
        elif isinstance(value,list):
            for i,v in enumerate(value):walk(v,path+'.'+str(i))
        elif isinstance(value,(int,float)) and not isinstance(value,bool):
            if not nonnegative(value):errors.append('SIDECAR_NUMERIC_RANGE: '+path)
    walk(layout,'layout');walk(motion,'motion')
    if layout['unit']!='logical-pt':errors.append('LAYOUT_UNIT')
    if layout['grid']!=2:errors.append('LAYOUT_GRID_REQUIRES_REVIEW')
    if layout['shadow']!='none':errors.append('SHADOW_POLICY_VIOLATION')
    for key in ['minimum','default']:
        if len(layout['window'][key])!=2 or not all(nonnegative(v) and v>0 for v in layout['window'][key]):errors.append('WINDOW_SIZE: '+key)
    if any(a>b for a,b in zip(layout['window']['minimum'],layout['window']['default'])):errors.append('WINDOW_MINIMUM_EXCEEDS_DEFAULT')
    for key in ['sidebar','inspector']:
        v=layout[key]
        if not 0<v['min']<=v['default']<=v['max']:errors.append('PANE_RANGE: '+key)
    if not 0<layout['composer']['min_height']<=layout['composer']['max_height']:errors.append('COMPOSER_RANGE')
    b=layout['breakpoints']
    if not layout['window']['minimum'][0]<=b['two_panes_min']<b['three_panes_min']:errors.append('BREAKPOINT_ORDER')
    layers=list(layout['layers'].values())
    if any(type(v) is not int for v in layers) or layers!=sorted(set(layers)):errors.append('LAYER_TYPE_OR_ORDER')
    for key in ['backing_scales','text_scales']:
        vals=layout['gallery'][key]
        if not vals or any(not nonnegative(v) or v<=0 for v in vals) or len(vals)!=len(set(vals)):errors.append('SCALE_RANGE: '+key)
    for name in layout['density'].values():
        if name.startswith('p-'):errors.append('DENSITY_USES_PRIMITIVE: '+name)
        try:resolve(tokens,'size',name)
        except (KeyError,ValueError) as e:errors.append(str(e))
    if motion['duration']!={'instant':0.,'press':.07,'stateChange':.15,'geometry':.12,'listExpansion':.3,'expansion':.5}:errors.append('MOTION_BASELINE_CHANGED_WITHOUT_REVIEW')
    easing=motion['easing']['standard']
    if len(easing)!=4 or any(not nonnegative(v) or v>1 for v in easing):errors.append('EASING_RANGE')
    if not motion['accessibility']['reduce_motion'].startswith('all-durations-zero'):errors.append('REDUCE_MOTION_CONTRACT')
    return errors

def recipe_document(c: dict) -> str:
    return f"""# {c['name']} · Consumer Recipe

稳定 ID：`{c['id']}`。状态：**{c['status']}；三层运行时证据均未验证**。{'可选扩展；不是首轮实施前置条件。' if c['optional'] else '核心工作台使用规范。'}

负责人角色：`{c['owner_role']}`；实际维护者未指派。所属 crate：`{c['crate']}`。不得放入通用组件 crate 或借此引入模型/网络依赖。

## API 与用法

```text
{c['api']}
```

使用：{c['usage']}。

禁区：{c['prohibited']}。

组合组件：{', '.join('`'+x+'`' for x in c['component_ids'])}。

## 状态、主题与证据

领域状态：{', '.join(c['states'])}。这些不是 hover/focus 的替代；子组件继续逐项满足通用组件状态和组合契约。

主题：{', '.join(c['themes'])}；三个 Gallery 品牌夹具全部覆盖，规则见 `../BRANDING.md`。夹具：{', '.join(c['fixtures'])}。具体颜色、尺寸必须来自 `../../design/tokens/`，尺寸超出可用空间时重排，不剪掉操作。

拟定稳定地址：`tekes-gallery://{c['gallery_slug']}`。当前不是可打开的原生页面。文档与期望用例由 `recipes.json` 生成，不能当作实际组件展示。

参考：{', '.join(c['source_ids'])}；读取范围与证据强度见 `../../research/open-claude-ui.lock.json`。清单级参考不代表复核过源文件实现；适配决定见 `../ALIGNMENT.md`。

真实鼠标、键盘、IME、VoiceOver 和消费者业务 payload 必须独立验收。既有输入、权限、发送偏好和历史分组不得在视觉迁移时静默改变。运行时证据字段：`{json.dumps(c['evidence'],ensure_ascii=False)}`。
"""

def alignment_errors(tokens: dict, registry: dict, recipes=None, contracts=None, interaction=None, lock=None, check_docs=True):
    recipes=read('design/registry/recipes.json') if recipes is None else recipes
    contracts=read('design/component-contracts.json') if contracts is None else contracts
    interaction=read('design/interaction-contracts.json') if interaction is None else interaction
    lock=read('research/open-claude-ui.lock.json') if lock is None else lock
    errors=[]
    errors += ['RECIPE_SCHEMA: '+e.message for e in jsonschema.Draft202012Validator(read('design/schemas/recipes.schema.json')).iter_errors(recipes)]
    if errors:return errors
    ids={x['id'] for x in registry['components']};fixtures={x['id'] for x in read('design/quality/fixtures.json')['fixtures']}
    source_ids={x['id'] for x in lock['files']}
    if len(source_ids)!=len(lock['files']):errors.append('DUPLICATE_REFERENCE_SOURCE')
    if not re.fullmatch(r'[a-f0-9]{40}',lock.get('reference_commit','')):errors.append('UNPINNED_REFERENCE')
    for source in lock['files']:
        if not re.fullmatch(r'[a-f0-9]{40}',source['git_blob_sha']):errors.append('MISSING_REFERENCE_BLOB: '+source['id'])
        if '/blob/'+lock['reference_commit']+'/' not in source['url']:errors.append('FLOATING_REFERENCE: '+source['id'])
        if source['inspection'] not in ['source-read','author-analysis-read','inventory-and-documentation-only']:errors.append('REFERENCE_EVIDENCE_SCOPE: '+source['id'])
    rids=set();slugs=set()
    for c in recipes['recipes']:
        if c['id'] in rids or c['gallery_slug'] in slugs:errors.append('DUPLICATE_RECIPE: '+c['id'])
        rids.add(c['id']);slugs.add(c['gallery_slug'])
        if not set(c['component_ids'])<=ids:errors.append('UNKNOWN_RECIPE_COMPONENT: '+c['id'])
        if not set(c['source_ids'])<=source_ids:errors.append('UNKNOWN_RECIPE_SOURCE: '+c['id'])
        if set(c['themes'])!=set(THEMES):errors.append('MISSING_RECIPE_THEME: '+c['id'])
        if not set(c['fixtures'])<=fixtures:errors.append('UNKNOWN_RECIPE_FIXTURE: '+c['id'])
        if c['status'] in ['beta','stable'] and (not c['owner_handle'] or 'consumer-defined' in c['crate']):errors.append('FALSE_READY_RECIPE: '+c['id'])
        if check_docs:
            p=ROOT/c['spec']
            if not p.exists() or p.read_text()!=recipe_document(c):errors.append('STALE_RECIPE_DOCUMENT: '+c['id'])
    if check_docs:
        expected={str(ROOT/c['spec']) for c in recipes['recipes']};actual={str(p) for p in (ROOT/'docs/recipes').glob('*.md')}
        if expected!=actual:errors.append('ORPHAN_OR_MISSING_RECIPE_DOCUMENT')
    for page in read('design/registry/pages.json')['pages']:
        if not set(page.get('recipe_ids',[]))<=rids:errors.append('UNKNOWN_PAGE_RECIPE: '+page['id'])
    key_family={'radius':'radius','inset':'spacing','inline_inset':'spacing','height':'size','focus_width':'stroke','default_hit':'size','compact_hit':'size','icon':'size','track':'color','thumb':'color','surface':'color','outline':'color','border':'color','focus':'color','header_separator':'color','rest':'color','hover':'color','pressed':'color','foreground':'color'}
    text_styles={x['name'] for g in tokens['type']['groups'] for x in g['styles']}
    def check_values(obj,path):
        if isinstance(obj,dict):
            for k,val in obj.items():
                if k in key_family and isinstance(val,str):
                    if val.startswith('p-'):errors.append('CONTRACT_PRIMITIVE_LEAK: '+path+'.'+k)
                    try:resolve(tokens,key_family[k],val)
                    except (ValueError,KeyError):errors.append('UNKNOWN_CONTRACT_TOKEN: '+path+'.'+k+'='+val)
                if k=='text_style' and val not in text_styles:errors.append('UNKNOWN_CONTRACT_TEXT_STYLE: '+str(val))
                if k=='source_ids' and not set(val)<=source_ids:errors.append('UNKNOWN_CONTRACT_SOURCE: '+path)
                check_values(val,path+'.'+k)
        elif isinstance(obj,list):
            for val in obj:check_values(val,path)
    for cid,contract in contracts['components'].items():
        if cid not in ids:errors.append('UNKNOWN_CONTRACT_COMPONENT: '+cid)
        check_values(contract,cid)
    if interaction.get('unit')!='logical-pt' or interaction.get('time_unit')!='seconds':errors.append('INTERACTION_UNIT')
    if not interaction['ime']['blocks_all_submit_paths'] or not interaction['ime']['preserve_original_payload']:errors.append('IME_OR_PAYLOAD_GUARD_REMOVED')
    if interaction['ime']['priority'][0]!='platform-marked-text':errors.append('IME_PRIORITY')
    if interaction['scroll']['reading_history_latched'] is not True:errors.append('SCROLL_HISTORY_LATCH_REMOVED')
    if not 0<interaction['scroll']['bottom_tolerance']<interaction['scroll']['jump_control_threshold']:errors.append('SCROLL_THRESHOLD_ORDER')
    if not 0<interaction['scroll']['cooldown']<=1:errors.append('SCROLL_COOLDOWN')
    if interaction['drag']['dead_zone']!=4 or not interaction['drag']['drag_alternative_required']:errors.append('DRAG_CONTRACT')
    if interaction['native']['auto_focus_background_window']:errors.append('BACKGROUND_FOCUS_STEAL')
    motion=tomllib.loads((ROOT/'design/tokens/motion.toml').read_text())
    for name,setting in motion['spring'].items():
        for key in ['stiffness','damping','mass']:
            if isinstance(setting[key],bool) or not isinstance(setting[key],(int,float)) or setting[key]<=0:errors.append('SPRING_PARAMETER: '+name+'.'+key)
    if motion['policy']['shadow'] or motion['policy']['text_blur'] or motion['policy']['idle_animation']:errors.append('MOTION_POLICY')
    if check_docs:
        reqs=read('design/quality/requirements-traceability.json')['requirements']
        if {x['id'] for x in reqs}!={'DS-'+str(i).zfill(2) for i in range(1,17)}:errors.append('REQUIREMENT_COVERAGE')
        for req in reqs:
            for path in req['artifacts']:
                if not (ROOT/path.split('#')[0]).exists():errors.append('MISSING_REQUIREMENT_ARTIFACT: '+path)
    return errors

def hash_errors():
    path=ROOT/'manifest.json'
    if not path.exists():return ['MISSING_MANIFEST']
    m=read('manifest.json');errors=[]
    for path,expected in m['files'].items():
        p=ROOT/path
        if not p.exists():errors.append('MISSING_FILE: '+path);continue
        if hashlib.sha256(p.read_bytes()).hexdigest()!=expected:errors.append('HASH_MISMATCH: '+path)
    aggregate=hashlib.sha256('\n'.join(f'{p}\0{h}' for p,h in sorted(m['files'].items())).encode()).hexdigest()
    if aggregate!=m['bundle_source_sha256']:errors.append('MANIFEST_AGGREGATE_MISMATCH')
    actual={str(p.relative_to(ROOT)) for p in ROOT.rglob('*') if p.is_file() and str(p.relative_to(ROOT))!='manifest.json' and not str(p.relative_to(ROOT)).startswith('reports/') and '__pycache__' not in p.parts}
    if set(m['files'])!=actual:errors.append('UNREGISTERED_OR_MISSING_SOURCE_FILE')
    return errors

def self_test():
    tokens=read('design/tokens/tokens.json');metadata=read('design/tokens/token-metadata.json');registry=read('design/registry/components.json')
    checks=[]
    def token_case(name,change):
        candidate=copy.deepcopy(tokens);change(candidate)
        errors=token_errors(candidate,metadata);checks.append({'name':name,'status':'pass' if errors else 'fail'})
    token_case('reject missing dark-ic',lambda t:next(x for x in t['color']['tokens'] if isinstance(x['value'],dict))['value'].pop('dark-ic'))
    token_case('reject alias cycle',lambda t:t['color']['tokens'][1].update(value='{'+t['color']['tokens'][1]['name']+'}'))
    token_case('reject orphan alias',lambda t:t['spacing']['tokens'][-1].update(value='{missing-token}'))
    token_case('reject wrong numeric type',lambda t:t['size']['tokens'][0].update(value='28px'))
    token_case('reject negative dimension',lambda t:t['size']['tokens'][0].update(value=-1))
    token_case('reject duplicate name',lambda t:t['color']['tokens'].append(copy.deepcopy(t['color']['tokens'][0])))
    bad=copy.deepcopy(registry);bad['components'][0]['themes'].remove('dark')
    checks.append({'name':'reject missing component theme','status':'pass' if registry_errors(bad,False) else 'fail'})
    bad=copy.deepcopy(tokens)
    next(t for t in bad['color']['tokens'] if t['name']=='p-primary-text')['value']['light']='#FAF9F6'
    checks.append({'name':'reject low contrast','status':'pass' if any(r['status']=='fail' for r in contrast_results(bad)) else 'fail'})
    layout=read('design/layout.json');motion=tomllib.loads((ROOT/'design/tokens/motion.toml').read_text())
    bad=copy.deepcopy(layout);bad['unit']='device-px'
    checks.append({'name':'reject invalid layout unit','status':'pass' if sidecar_errors(bad,motion,tokens) else 'fail'})
    bad=copy.deepcopy(layout);bad['layers']['popover']=30.5
    checks.append({'name':'reject fractional layer','status':'pass' if sidecar_errors(bad,motion,tokens) else 'fail'})
    bad=copy.deepcopy(motion);bad['easing']['standard']=[2,0,0,1]
    checks.append({'name':'reject invalid easing control point','status':'pass' if sidecar_errors(layout,bad,tokens) else 'fail'})
    for name,kind,change in [
        ('reject unknown recipe component','recipes',lambda x:x['recipes'][0]['component_ids'].append('tui.missing')),
        ('reject missing recipe theme','recipes',lambda x:x['recipes'][0]['themes'].remove('dark-ic')),
        ('reject missing typed component token','contracts',lambda x:x['components']['tui.button']['sizes']['md'].update(radius='missing-radius')),
        ('reject removed composition guard','interaction',lambda x:x['ime'].update(blocks_all_submit_paths=False)),
        ('reject removed history latch','interaction',lambda x:x['scroll'].update(reading_history_latched=False)),
        ('reject floating reference','lock',lambda x:x['files'][0].update(url=x['files'][0]['url'].replace(x['reference_commit'],'main'))),
    ]:
        paths={'recipes':'design/registry/recipes.json','contracts':'design/component-contracts.json','interaction':'design/interaction-contracts.json','lock':'research/open-claude-ui.lock.json'}
        candidate=read(paths[kind]);change(candidate)
        found=alignment_errors(tokens,registry,check_docs=False,**{kind:candidate})
        checks.append({'name':name,'status':'pass' if found else 'fail'})
    from context import source_errors, expected_artifacts as context_artifacts, freshness_errors as context_freshness
    bad=read('design/quality/reference-adoption.json');bad['decisions'][0]['source_ids'].append('NONEXISTENT-SOURCE')
    checks.append({'name':'reject unknown source in reference adoption','status':'pass' if source_errors(bad) else 'fail'})
    bad=read('design/quality/reference-adoption.json');bad['decisions'][0]['artifacts'].append('../outside-artifact')
    checks.append({'name':'reject source adoption path escaping bundle','status':'pass' if source_errors(bad) else 'fail'})
    generated=context_artifacts();generated['docs/AGENT-HANDOFF.md']=b'intentionally stale'
    checks.append({'name':'reject stale generated handoff context','status':'pass' if context_freshness(generated) else 'fail'})
    from test_brand import run_tests
    brand_checks=run_tests()
    dump('reports/brand-tests.json',brand_checks)
    return checks + brand_checks

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--release',action='store_true');parser.add_argument('--self-test',action='store_true');parser.add_argument('--write-report',action='store_true');args=parser.parse_args()
    if args.self_test:
        result=self_test();print(json.dumps(result,ensure_ascii=False,indent=2))
        if args.write_report:dump('reports/negative-tests.json',result)
        return 0 if all(x['status']=='pass' for x in result) else 1
    tokens=read('design/tokens/tokens.json');metadata=read('design/tokens/token-metadata.json');registry=read('design/registry/components.json')
    errors=token_errors(tokens,metadata)+registry_errors(registry)+hash_errors()
    motion=tomllib.loads((ROOT/'design/tokens/motion.toml').read_text())
    errors += sidecar_errors(read('design/layout.json'),motion,tokens)
    errors += alignment_errors(tokens,registry)
    from brand import validate_all
    try:
        brand_errors, brand_report = validate_all(args.write_report)
        errors += brand_errors
        from generate import case_payloads, documentation_errors
        errors += documentation_errors()
        for path,expected in case_payloads().items():
            if not (ROOT/path).exists() or read(path)!=expected:
                errors.append('STALE_EXPECTED_CASES: '+path)
    except (ValueError, KeyError, TypeError) as error:
        errors.append('BRAND_VALIDATION: '+str(error))
        brand_report = {'status':'fail','error':str(error)}
    from context import validate_all as validate_context
    try:
        context_errors,context_report=validate_context(args.write_report)
        errors += context_errors
    except (OSError,ValueError,KeyError,TypeError) as error:
        errors.append('CONTEXT_VALIDATION: '+str(error))
        context_report={'status':'fail','error':str(error)}
    contrasts=[]
    if not token_errors(tokens,metadata):
        contrasts=contrast_results(tokens)
        errors += [f"CONTRAST: {c['theme']} {c['foreground']}/{c['background']} {c['ratio']:.6f} < {c['minimum']}" for c in contrasts if c['status']=='fail']
    scope='proposal-static'
    if args.release:
        scope='release-readiness';r=read('design/quality/release-readiness.json')
        if not r['source_revision'] or not r['source_export_sha256']:errors.append('SOURCE_PROVENANCE_UNVERIFIED')
        for k in ['artifact_export_compatibility','runtime_implementation','gallery_native_runtime','real_environment','icon_originality_review','owner_assignments','consumer_brand_approval','native_theme_switch','brand_gallery_evidence']:
            if r[k]!='pass':errors.append(k.upper()+'_UNVERIFIED')
        for check in read('design/quality/craft-review.json')['checks']:
            if check['status']!='pass' or not check.get('evidence_uri') or not check.get('reviewer'):
                errors.append('CRAFT_REVIEW_UNVERIFIED: '+check['id'])
        for c in registry['components']:
            if c['status']!='stable' :errors.append('COMPONENT_NOT_STABLE: '+c['id'])
    report={'scope':scope,'status':'pass' if not errors else 'fail','themes':THEMES,'token_count':sum(len(v['tokens']) for k,v in tokens.items() if k!='type'),'text_style_count':sum(len(g['styles']) for g in tokens['type']['groups']),'component_count':len(registry['components']),'recipe_count':len(read('design/registry/recipes.json')['recipes']),'declared_contrast_checks':len(contrasts),'contrast_failures':sum(c['status']=='fail' for c in contrasts),'errors':errors,'not_verified':['Private source artifact revision and original export','Rust APIs and native Gallery','actual rendered pixels and final compositing','real mouse/keyboard/IME/VoiceOver','actual application coverage and performance','icon originality, missing assets and licenses'],'bundle_source_sha256':read('manifest.json')['bundle_source_sha256'] if (ROOT/'manifest.json').exists() else None}
    report['brand_validation'] = brand_report
    report['context_validation'] = context_report
    print(json.dumps(report,ensure_ascii=False,indent=2))
    if args.write_report:
        dump('reports/release-readiness.json' if args.release else 'reports/proposal-validation.json',report)
        if not args.release:dump('reports/contrast-results.json',contrasts)
    return 0 if not errors else 1
if __name__=='__main__':
    try:sys.exit(main())
    except (OSError,ValueError,KeyError,jsonschema.SchemaError) as e:
        print(f'VALIDATION_ERROR: {e}',file=sys.stderr);sys.exit(2)
