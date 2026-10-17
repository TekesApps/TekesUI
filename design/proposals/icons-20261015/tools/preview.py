#!/usr/bin/env python3
"""Local comparison only: keeps private legacy SVGs out of the public proposal folder."""
from pathlib import Path
import json, argparse, hashlib, shutil
parser=argparse.ArgumentParser();parser.add_argument('--legacy',type=Path,required=True);parser.add_argument('--out',type=Path,required=True);a=parser.parse_args()
root=Path(__file__).resolve().parents[1]; manifest=json.loads((root/'manifest.json').read_text())
a.out.mkdir(parents=True,exist_ok=True); combined=a.out/'gallery-svg';combined.mkdir(exist_ok=True)
retired={'tekes-annotation','tekes-computer-use','tekes-perm-full-access','tekes-perm-read-only','tekes-perm-workspace-write'}
sourcehash={};old={}
for d in sorted(a.legacy.glob('*.imageset')):
 name=d.stem
 if name in retired:continue
 p=d/f'{name}.svg';svg=p.read_text()
 assert 'c2pa' not in svg.lower()
 sourcehash[name]=hashlib.sha256(p.read_bytes()).hexdigest()
 old[name]=svg.replace('#000','currentColor')
 (combined/f'{name}.svg').write_text(svg)
assert len(old)==133
items=[]
for item in manifest['items']:
 p=root/'svg'/f"{item['name']}.svg"
 svg=p.read_text();shutil.copyfile(p,combined/p.name)
 items.append({**item,'svg':svg,'before':old.get(item['name'],'')})
assert len(list(combined.glob('*.svg')))==176
# Includes the previous full catalogue for an honest whole-set audit, not only chosen examples.
data={'items':items,'legacy':[{'name':n,'svg':s} for n,s in old.items()]}
html='''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Tekes 图标工作台</title>
<style>*{box-sizing:border-box}body{margin:0;background:#f8f8f7;color:#202322;font:14px/1.55 -apple-system,BlinkMacSystemFont,sans-serif}body.dark{background:#141616;color:#e9edeb}button,input{font:inherit;color:inherit;background:transparent;border:1px solid #a3aaa6;border-radius:6px;padding:8px 12px}button{cursor:pointer}button.active{background:#202b25;color:#fff}body.dark button.active{background:#dfe7e1;color:#162019}header{padding:24px 36px;border-bottom:1px solid #c2c8c3;position:sticky;top:0;background:inherit;z-index:2}h1{font-size:25px;letter-spacing:-.6px;font-weight:600;margin:0 0 4px}.sub{color:#707871}body.dark .sub{color:#a0aaa3}nav{display:flex;flex-wrap:wrap;gap:8px;margin-top:16px}header .tools{display:flex;gap:8px;margin-top:14px}input{width:380px}main{padding:28px 36px}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,330px),1fr));gap:16px}.grid.comparison{grid-template-columns:repeat(auto-fit,minmax(min(100%,600px),1fr))}.card{border:1px solid #bcc3bd;border-radius:10px;padding:20px;min-width:0}.name{font-weight:600;font-size:15px}.id{font:11px ui-monospace,monospace;overflow-wrap:anywhere;margin-top:2px}.preview{display:flex;align-items:center;gap:25px;height:82px}.glyph{display:flex;align-items:center;justify-content:center;min-width:26px;flex-direction:column;gap:8px}.glyph span{font-size:10px;color:#7b847e}.glyph svg{flex:none;display:block}.beforeafter{display:grid;grid-template-columns:1fr 1fr;gap:20px}.beforeafter .preview{gap:12px}.tag{font-size:11px;margin-top:12px}.desc{margin-top:12px;font-size:12px;min-height:38px}.oldtitle{font-size:11px;color:#7b847e}#count{margin:0 0 16px;color:#7b847e}footer{padding:20px 36px;font-size:12px;color:#7b847e}@media(max-width:420px){.beforeafter{grid-template-columns:1fr}.tools{flex-wrap:wrap}input{width:100%}}</style>
<header><h1>Tekes 图标工作台</h1><div class="sub">三应用共用 · 24×24 · 1.5 pt · 圆头描边 · 可审查 SVG 提案</div><nav id="nav"></nav><div class="tools"><input id="query" placeholder="按名称或用途搜索"><button id="theme">切换深浅色</button></div></header><main><p id="count"></p><div class="grid" id="grid"></div></main><footer>38 项新增 · 8 项优化 · 5 项独立重画。提案未替换生产应用；旧稿仅本机对比。12 / 16 / 24 为实际逻辑尺寸，48 为放大检查。</footer>
<script type="application/json" id="data">__DATA__</script><script>
const data=JSON.parse(document.querySelector('#data').textContent);let group='all';
const groups=[['all','全部提案 51'],['shared','共用操作 10'],['cad','TekesDXF 14'],['drama','TekesDrama 14'],['refine','旧稿优化 8'],['rebuild','独立重画 5'],['legacy','原完整集 133']];
const esc=s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
function stripSize(svg,size){return svg.replace(/width="24"/,`width="${size}"`).replace(/height="24"/,`height="${size}"`)}
function preview(svg){return '<div class="preview">'+[12,16,24,48].map(n=>`<div class="glyph">${stripSize(svg,n)}<span>${n}</span></div>`).join('')+'</div>'}
function render(){document.querySelector('#grid').classList.toggle('comparison',group==='refine');let q=document.querySelector('#query').value.toLowerCase();let source=group==='legacy'?data.legacy:data.items;let list=source.filter(i=>(group==='all'||group==='legacy'||i.group===group)&&[i.name,i.label,i.usage,i.reason].join(' ').toLowerCase().includes(q));document.querySelector('#count').textContent=`${list.length} 项 · ${group==='refine'?'左旧稿 / 右优化稿 · 同尺寸、同颜色对比':'逐项显示 12 / 16 / 24 / 48 pt'}`;document.querySelector('#grid').innerHTML=list.map(i=>`<article class="card"><div class="name">${esc(i.label??i.name)}</div><div class="id sub">${esc(i.name)}</div>${i.before&&group==='refine'?`<div class="beforeafter"><div><div class="oldtitle">原稿</div>${preview(i.before)}</div><div><div class="oldtitle">优化提案</div>${preview(i.svg)}</div></div>`:preview(i.svg)}<div class="desc">${esc(i.reason||i.usage||'原完整图标集，只读参照')}</div></article>`).join('');document.querySelectorAll('nav button').forEach(b=>b.classList.toggle('active',b.dataset.id===group));}
document.querySelector('#nav').innerHTML=groups.map(([id,label])=>`<button data-id="${id}">${label}</button>`).join('');document.querySelector('#nav').onclick=e=>{if(e.target.dataset.id){group=e.target.dataset.id;render()}};document.querySelector('#query').oninput=render;document.querySelector('#theme').onclick=()=>document.body.classList.toggle('dark');render();
</script></html>'''
(a.out/'index.html').write_text(html.replace('__DATA__',json.dumps(data,ensure_ascii=False).replace('</','<\\/')))
(a.out/'source-hashes.json').write_text(json.dumps(sourcehash,indent=2)+'\n')
print(f'{a.out}/index.html | local Gallery overlay: 176 SVG + 10 built-in = 186')
