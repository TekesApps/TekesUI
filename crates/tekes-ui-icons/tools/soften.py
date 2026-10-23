#!/usr/bin/env python3
"""Reproduce the reviewed Gold7 artwork from the preserved pre-change SVGs.

Only named contours are eligible. Curves, endpoints, element/contour order,
canonical names, paint and 24x24 artboards retain their original ownership.
"""
from pathlib import Path
import argparse,hashlib,json,math,re,xml.etree.ElementTree as ET
ROOT=Path(__file__).resolve().parents[3]
REVIEW=ROOT/'design/reviews/icon-gold7-57'
ASSETS=ROOT/'crates/tekes-ui-icons/assets'
APPROVED={'back','forward','settings','folder','file-generic','error'}
# Explicit eligibility, not a blanket transformation of technical/semantic tips.
PLAN={
 'access-full':[0], 'access-workspace':[0], 'annotate':[0,1], 'collapse':[0], 'expand':[0],
 'command':[0], 'shell':[0], 'desktop-control':[0],
 'diff-annotation':[0,1,4], 'drama-collaboration':[0,1],
 'drama-directing':[3], 'drama-editing':[0], 'drama-character':[1], 'drama-script':[0,1],
 'file-link':[0,1], 'files':[0,1], 'filter':[0], 'fullscreen':[0,2],
 'image':[0], 'memorize':[0], 'note':[0], 'plugin':[2], 'plugins':[2],
 'popup-menu':[0,1], 'queued-input':[3], 'recall':[0],
 'redo':[0], 'rewind':[0], 'review-plan':[0,1], 'router':[1,2], 'steer':[1],
 'steering':[0,2], 'undo':[0],
}
DOCUMENTS={'file-added','file-rich','file-text','read','report'}
FOLDERS={'new-project','untracked-folder'}
PRECISION={'cad-arc','cad-block','cad-compare','cad-crossing-select','cad-dimension',
 'cad-export','cad-layers','cad-layout','cad-line','cad-pdf-import','cad-polyline',
 'cad-snap','cad-table-extract','cad-window-select','profile','drama-delivery','drama-shot',
 'flash','flash-fill','skill','skills','web-fetch','edit','write','oauth-key'}
def sha(s):return hashlib.sha256(s.encode()).hexdigest()
def fmt(v):return f'{v:.4f}'.rstrip('0').rstrip('.') if v else '0'
def xy(p):return fmt(p[0])+' '+fmt(p[1])
def contours(d):
    tokens=re.findall(r'[a-zA-Z]|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?',d)
    sizes={'M':2,'L':2,'H':1,'V':1,'C':6,'S':4,'Q':4,'T':2,'A':7}
    out=[];segments=[];pos=(0.,0.);start=pos;cmd=None;i=0
    while i<len(tokens):
        if tokens[i].isalpha():cmd=tokens[i];i+=1
        upper=cmd.upper()
        if upper=='Z':
            segments.append(('Z',[],pos,start));pos=start;cmd=None
            out.append(segments);segments=[];continue
        n=sizes[upper];values=list(map(float,tokens[i:i+n]));assert len(values)==n;i+=n
        relative=cmd.islower();v=values[:]
        if upper=='H':v=[v[0]+(pos[0] if relative else 0),pos[1]];upper='L'
        elif upper=='V':v=[pos[0],v[0]+(pos[1] if relative else 0)];upper='L'
        elif relative:
            if upper=='A':v[-2]+=pos[0];v[-1]+=pos[1]
            else:
                v=[x+pos[j%2] for j,x in enumerate(v)]
        end=(v[-2],v[-1])
        if upper=='M':
            if segments:out.append(segments);segments=[]
            start=end
        segments.append((upper,v,pos,end));pos=end
        if cmd.upper()=='M':cmd='l' if relative else 'L'
    if segments:out.append(segments)
    return out
def soften_contour(segs,radius):
    if segs[-1][0]=='Z' and all(s[0] in {'M','L','Z'} for s in segs):
        points=[s[3] for s in segs if s[0]!='Z']
        if points[-1]==points[0]:points.pop()
        bends=[]
        for i,q in enumerate(points):
            p,z=points[i-1],points[(i+1)%len(points)];la=math.dist(p,q);lb=math.dist(q,z)
            if min(la,lb)<.5:bends.append((q,q));continue
            u=((p[0]-q[0])/la,(p[1]-q[1])/la);v=((z[0]-q[0])/lb,(z[1]-q[1])/lb)
            dot=u[0]*v[0]+u[1]*v[1]
            t=0 if dot<-.985 or dot>.7 else min(radius,la*.32,lb*.32)
            bends.append(((q[0]+u[0]*t,q[1]+u[1]*t),(q[0]+v[0]*t,q[1]+v[1]*t)))
        result='M'+xy(bends[0][1])
        for i in list(range(1,len(points)))+[0]:
            a,b=bends[i];result+=' L'+xy(a)
            if a!=b:result+=' Q'+xy(points[i])+' '+xy(b)
        return result+' Z'
    # A conservative contour pass: round only consecutive straight segments.
    # Existing curve junctions, closed-contour seam, and endpoints stay exact.
    result='';adjust={};seam=None
    if segs[-1][0]=='Z' and segs[1][0]=='L':
        segs=list(segs)
        if segs[-2][3]!=segs[0][3]:
            segs.insert(-1,('L',list(segs[0][3]),segs[-2][3],segs[0][3]))
        if segs[-2][0]=='L':
            p,q,z=segs[-2][2],segs[0][3],segs[1][3];la=math.dist(p,q);lb=math.dist(q,z)
            if min(la,lb)>=.5:
                u=((p[0]-q[0])/la,(p[1]-q[1])/la);v=((z[0]-q[0])/lb,(z[1]-q[1])/lb)
                dot=u[0]*v[0]+u[1]*v[1]
                if -.985<=dot<=.7:
                    t=min(radius,la*.32,lb*.32)
                    seam=((q[0]+u[0]*t,q[1]+u[1]*t),q,(q[0]+v[0]*t,q[1]+v[1]*t))
    for i in range(1,len(segs)-1):
        a,b=segs[i],segs[i+1]
        if a[0]!='L' or b[0]!='L':continue
        p,q,z=a[2],a[3],b[3];la=math.dist(p,q);lb=math.dist(q,z)
        if min(la,lb)<.5:continue
        u=((p[0]-q[0])/la,(p[1]-q[1])/la);v=((z[0]-q[0])/lb,(z[1]-q[1])/lb)
        dot=u[0]*v[0]+u[1]*v[1]
        if dot<-.985 or dot>.7:continue # straight joins or identifying acute tips
        t=min(radius,la*.32,lb*.32)
        adjust[i]=((q[0]+u[0]*t,q[1]+u[1]*t),q,(q[0]+v[0]*t,q[1]+v[1]*t))
    for i,(cmd,v,_,_) in enumerate(segs):
        if seam and i==0:result+=' M'+xy(seam[2])
        elif seam and i==len(segs)-2:result+=' L'+xy(seam[0])+' Q'+xy(seam[1])+' '+xy(seam[2])
        elif i in adjust:
            a,q,b=adjust[i];result+=' L'+xy(a)+' Q'+xy(q)+' '+xy(b)
        else:result+=' '+cmd+(' '.join(fmt(x) for x in v) if v else '')
    return result.strip()
def change_paths(svg,indices):
    # Preserve attribute/element order (animation layer references use it).
    index=0
    def one(m):
        nonlocal index
        cs=contours(m.group(1));pieces=[]
        for c in cs:
            selected=index in indices
            pieces.append(soften_contour(c,1.2 if len(c)<=4 else .8) if selected else ' '.join(s[0]+' '.join(fmt(x) for x in s[1]) for s in c))
            index+=1
        # Keep exact bytes of untouched path elements.
        active=any(i in indices for i in range(index-len(cs),index))
        return 'd="'+' '.join(pieces)+'"' if active else m.group(0)
    return re.sub(r'\bd="([^"]*)"',one,svg)
def render_v2_geometry(name,original):
    short=name.removeprefix('tekes-')
    if short in APPROVED:return (REVIEW/'approved-v2'/f'{name}.svg').read_text(),'approved-v2','User-approved representative geometry, exact bytes.'
    if short in DOCUMENTS:
        base=(REVIEW/'before/tekes-file-generic.svg').read_text()
        new=(REVIEW/'approved-v2/tekes-file-generic.svg').read_text()
        old_d=re.search(r'd="([^"]+)"',base)[1];new_d=re.search(r'd="([^"]+)"',new)[1]
        assert old_d in original,name
        return original.replace(old_d,new_d),'document-family','Shared approved file silhouette/fold; interior meaning retained.'
    if short in FOLDERS:
        old_d=re.search(r'd="([^"]+)"',(REVIEW/'before/tekes-folder.svg').read_text())[1]
        new_d=re.search(r'd="([^"]+)"',(REVIEW/'approved-v2/tekes-folder.svg').read_text())[1]
        assert old_d in original,name
        return original.replace(old_d,new_d),'folder-family','Shared approved folder shoulders/tab; interior meaning retained.'
    if short=='folder-open':
        d=re.search(r'd="([^"]+)"',original)[1]
        new_d='M3 8 Q3 5.5 5.5 5.5 H8.35 Q8.85 5.5 9.2 5.92 L10.65 7.6 Q11 8 11.55 8 H18.5 Q21 8 21 10.5'
        return original.replace(d,new_d),'folder-family','Shared approved folder tab/shoulders; open front flap retained.'
    if short=='error-fill':
        old_d=re.search(r'd="([^"]+)"',(REVIEW/'before/tekes-error.svg').read_text())[1]
        new_d=re.search(r'd="([^"]+)"',(REVIEW/'approved-v2/tekes-error.svg').read_text())[1]
        return original.replace(old_d,new_d),'filled-pair','Match approved error exterior; even-odd interior cross and paint unchanged.'
    if short in PLAN:
        return change_paths(original,PLAN[short]),'rounded-line-junctions','Named ordinary contour joins softened; exact endpoints, curves and semantic detail retained.'
    if short in PRECISION:return original,'retained-precision','Identifying technical geometry, pointer/tool tip, spark/bolt or perspective edges require precision.'
    return original,'retained-existing','Existing curved/rounded contour or sparse round-ended marks already match; no forced redraw.'
def render(name,original):
    short=name.removeprefix('tekes-')
    approved=REVIEW/'approved-c'/f'{name}.svg'
    if approved.exists():return approved.read_text(),'approved-c','User-approved C: rounded geometry, optical breathing room and 1.25pt strokes; exact representative bytes.'
    new,rule,reason=render_v2_geometry(name,original)
    if short in DOCUMENTS:
        old_d=re.search(r'd="([^"]+)"',(REVIEW/'before/tekes-file-generic.svg').read_text())[1]
        new_d=re.search(r'd="([^"]+)"',(REVIEW/'approved-c/tekes-file-generic.svg').read_text())[1]
        new=original.replace(old_d,new_d);rule='c-document-family';reason='Approved C document shoulders/fold and wider curved lower corners; interior marks retained.'
    elif short in FOLDERS:
        old_d=re.search(r'd="([^"]+)"',(REVIEW/'before/tekes-folder.svg').read_text())[1]
        new_d=re.search(r'd="([^"]+)"',(REVIEW/'approved-c/tekes-folder.svg').read_text())[1]
        new=original.replace(old_d,new_d);rule='c-folder-family';reason='Approved C rounded folder body/tab; new-project mark and untracked dash contract retained.'
    elif short=='error-fill':
        old_d=re.search(r'd="([^"]+)"',(REVIEW/'before/tekes-error.svg').read_text())[1]
        new_d=re.search(r'd="([^"]+)"',(REVIEW/'approved-c/tekes-error.svg').read_text())[1]
        return original.replace(old_d,new_d),'c-filled-pair','Approved C error exterior; original even-odd cross, filled state, opacity and paint retained.'
    tree=ET.fromstring(new)
    stroked=any(c.get('stroke',tree.get('stroke'))!='none' for c in tree)
    if short.startswith('cad-'):
        return new,'optical-density-exception','Precision CAD geometry and 1.5pt strokes retained for 12pt control points, dashes and dense construction detail.'
    if not stroked:
        return new,'filled-silhouette-exception','Pure filled state has no stroke to lighten; retain full-alpha silhouette and counters.'
    if short=='flash-fill':
        return new,'filled-silhouette-exception','Filled lightning silhouette retains its sharp semantic points and original perimeter weight.'
    new=new.replace('stroke-width="1.5"','stroke-width="1.25"')
    return new,'c-'+rule,reason+' C optical weight is 1.25pt; color and alpha unchanged.'
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--check',action='store_true');args=ap.parse_args()
    rows=[]
    for source in sorted((REVIEW/'before').glob('*.svg')):
        original=source.read_text();new,rule,reason=render(source.stem,original)
        tree=ET.fromstring(new);oldtree=ET.fromstring(original)
        assert tree.get('viewBox')=='0 0 24 24' and tree.get('width')=='24' and tree.get('height')=='24'
        assert [c.tag for c in tree]==[c.tag for c in oldtree],source.name
        assert re.findall(r'opacity="[^"]*"',new)==re.findall(r'opacity="[^"]*"',original)
        assert 'filter=' not in new
        assert len(re.findall(r'[Mm]', ' '.join(c.get('d','') for c in tree)))==len(re.findall(r'[Mm]', ' '.join(c.get('d','') for c in oldtree))),source.name
        target=ASSETS/source.name
        if args.check:assert target.read_text()==new,source.name
        else:target.write_text(new)
        rows.append({'name':source.stem,'path':str(target.relative_to(ROOT)),'before_sha256':sha(original),'after_sha256':sha(new),'changed':new!=original,'rule':rule,'reason':reason,'canvas':[24,24],'viewBox':[0,0,24,24],'stroke_width':float(tree.get('stroke-width'))})
    report=REVIEW/'coverage.json'
    if not args.check:report.write_text(json.dumps({'issue':57,'approved_direction':'C; user explicitly approved full rollout, square canvas and full acceptance before closure','icons':rows},indent=2)+'\n')
    else:assert json.loads(report.read_text())['icons']==rows
    catalog_path=ASSETS.parent/'catalog.json';catalog=json.loads(catalog_path.read_text())
    if not args.check:catalog['strokeWidth']=1.25
    assert catalog['strokeWidth']==1.25
    expected={r['name']:r['after_sha256']for r in rows}
    assert {i['name']for i in catalog['icons']}==set(expected)
    for icon in catalog['icons']:
        if args.check:assert icon['sha256']==expected[icon['name']],icon['name']
        else:icon['sha256']=expected[icon['name']]
    if not args.check:catalog_path.write_text(json.dumps(catalog,indent=2)+'\n')
    style='//! Generated from catalog.json by tools/soften.py. Artwork metrics, not host layout.\n'
    style+=f'pub const REGULAR_STROKE_WIDTH: f32 = {catalog["strokeWidth"]};\n'
    style+=f'pub const ARTBOARD_SIZE: f32 = {float(catalog["viewBox"][2])};\n'
    style_path=ASSETS.parent/'src/style.rs'
    if args.check:assert style_path.read_text()==style
    else:style_path.write_text(style)
    print(f'{len(rows)} square canonical assets; {sum(r["changed"] for r in rows)} changed; {sum(not r["changed"] for r in rows)} explicitly retained')
if __name__=='__main__':main()
