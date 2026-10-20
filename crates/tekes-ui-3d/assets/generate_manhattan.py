"""MIT original, deterministic Manhattan-inspired diorama; no geographic survey data.
One indexed mesh batches architecture, facades, island and water for one draw.
Run with standard-library Python; output is a self-contained Blender-compatible GLB.
"""
from pathlib import Path
import json
import math
import random
import struct
import zlib
import binascii


def build():
    rng = random.Random(20261020)
    positions, normals, colors, uvs = [], [], [], []
    plain_indices, glass_indices = [], []
    indices = plain_indices
    occluders = []

    def linear(rgb):
        return [(v / 255 / 12.92 if v <= 10.31475 else ((v / 255 + .055) / 1.055) ** 2.4) for v in rgb] + [1.]

    def quad(points, rgb, facade=None):
        target = glass_indices if facade else plain_indices
        a, b, c, _ = points
        u, v = [b[i]-a[i] for i in range(3)], [c[i]-a[i] for i in range(3)]
        n = [u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0]]
        length = math.sqrt(sum(x*x for x in n))
        n = [x / length for x in n]
        start = len(positions)//3
        for i,point in enumerate(points):
            positions.extend(point); normals.extend(n); colors.extend(linear(rgb))
            uv = [(0,1),(1,1),(1,0),(0,0)][i]
            uvs.extend([(uv[0]*facade[0]+facade[2])/3., uv[1]*facade[1]] if facade else [0.,0.])
        target.extend([start, start+1, start+2, start, start+2, start+3])

    def box(x, z, w, d, bottom, height, rgb, windows=False):
        top = bottom+height
        if windows: occluders.append((x-w/2,x+w/2,z-d/2,z+d/2,top))
        # Counterclockwise outward faces. Facade cells share the same plane;
        # window colors are geometry attributes, not overlapping decals.
        for ax, az, bx, bz in [(x-w/2,z+d/2,x+w/2,z+d/2), (x+w/2,z+d/2,x+w/2,z-d/2),
                                  (x+w/2,z-d/2,x-w/2,z-d/2), (x-w/2,z-d/2,x-w/2,z+d/2)]:
            style = 1 if rgb[0] > rgb[2]+12 else (2 if abs(rgb[0]-rgb[2])<10 else 0)
            facade = (min(.995,math.hypot(bx-ax,bz-az)/2.56),min(.995,height/12.8),style) if windows else None
            quad([(ax,bottom,az),(bx,bottom,bz),(bx,top,bz),(ax,top,az)],rgb,facade)
        quad([(x-w/2,top,z+d/2),(x+w/2,top,z+d/2),(x+w/2,top,z-d/2),(x-w/2,top,z-d/2)],rgb)
        quad([(x-w/2,bottom,z-d/2),(x+w/2,bottom,z-d/2),(x+w/2,bottom,z+d/2),(x-w/2,bottom,z+d/2)],rgb)

    # Cropped harbor slab keeps the whole diorama compact in the orbit camera.
    box(0,0,36,14,-.65,.24,(26,73,94))
    for x in range(-17,18):
        for z in range(-6,7):
            # Low contrast water facets, no translucent sorting or per-frame CPU work.
            c=rng.choice([(38,93,112),(39,96,116),(40,99,118),(41,98,116)])
            quad([(x-.5,-.40,z+.5),(x+.5,-.40,z+.5),(x+.5,-.40,z-.5),(x-.5,-.40,z-.5)],c)
    # Island silhouette and terraced tapered southern tip.
    box(1,0,28,8.5,-.38,.6,(105,117,106))
    for x,w,d in [(-13.6,1.2,5.5),(-14.6,.8,4.3),(-15.3,.6,2.9),(-15.8,.4,1.4)]:
        box(x,0,w,d,-.38,.6,(104,123,108))
    box(6.9,0,5.4,1.55,.23,.035,(68,111,88)) # Central Park-inspired opening
    for side in [-1,1]:
        box(.5,side*4.0,29,.20,.23,.025,(190,177,145))
        for x in [-12,-9,-5,0,5,10]:
            box(x,side*4.65,.45,1.4,-.15,.38,(148,146,126))
    for z in [-2.62,-1.385,0,1.385,2.62]:
        box(.5,z,28.4,.25,.225,.02,(69,76,77))
        for x in range(-13,15): box(x,z,.25,.012,.246,.003,(202,193,155))
    for n in range(24):
        x=n*1.18-13.09
        box(x,0,.18,7.7,.225,.02,(76,81,80))
        for z in [-2.62,-1.385,0,1.385,2.62]:
            for dx in [-.05,.01,.07]:box(x+dx,z+.17,.025,.13,.247,.003,(210,210,196))
    palette=[(225,232,231),(235,225,206),(197,220,231),(219,226,225),(209,202,192),(227,214,190)]
    landmarks=[(-10.,.2,2.),(1.5,-.4,1.6),(4.,1.8,1.35),(7.,-2.1,1.1)]
    count=0
    for x in [i*1.18-12.5 for i in range(23)]:
        for z in [-3.15,-2.05,-.72,.72,2.05,3.15]:
            if 4.2<x<9.6 and abs(z)<1.: continue
            if any(abs(x-lx)<r and abs(z-lz)<r for lx,lz,r in landmarks): continue
            h=rng.uniform(1.0,3.0)
            h+=2.0*math.exp(-((x+10)/3)**2)+2.6*math.exp(-((x-2)/4)**2)
            w,d=rng.uniform(.62,.94),rng.uniform(.65,1.02)
            color=rng.choice(palette)
            box(x,z,w,d,.26,h,color,True)
            box(x,z,w*.86,d*.86,h+.26,.05,(154,157,151))
            for offset in [-.18,.16]:
                box(x+offset,z,.15,.19,h+.31,.10,(153,159,159))
            if rng.random()<.3: box(x,z+.20,.18,.18,h+.31,.28,(124,128,122))
            count+=1
    # Lower Manhattan glass taper, inspired by One World Trade Center.
    # Continuous tapered glass faces, without stair-stepped stacked boxes.
    for ax,az,bx,bz in [(-.85,.85,.85,.85),(.85,.85,.85,-.85),(.85,-.85,-.85,-.85),(-.85,-.85,-.85,.85)]:
        quad([(-10+ax,.26,.2+az),(-10+bx,.26,.2+bz),(-10+bx*.65,8.8,.2+bz*.65),(-10+ax*.65,8.8,.2+az*.65)],(200,228,240),(.66,.67,0))
    box(-10,.2,1.10,1.10,8.80,.10,(180,196,199))
    box(-10,.2,.045,.045,8.90,1.7,(191,202,202))
    # Art Deco setbacks and mast, inspired by the Empire State silhouette.
    bottom=.26
    for w,d,h in [(1.8,1.2,2.2),(1.48,1.05,3.6),(1.12,.88,2.1),(.74,.64,1.2),(.4,.4,.65)]:
        box(1.5,-.4,w,d,bottom,h,(238,230,210),True);bottom+=h
    box(1.5,-.4,.10,.10,bottom,1.6,(208,204,181))
    # Stepped silver crown and spire, Chrysler-inspired.
    bottom=.26
    for w,h in [(1.05,5.6),(.87,.65),(.67,.55),(.46,.45),(.25,.35)]:
        box(4,1.8,w,w,bottom,h,(220,230,230),True);bottom+=h
    box(4,1.8,.06,.06,bottom,1.1,(207,218,211))
    box(7,-2.1,.64,.64,.26,8.8,(235,221,194),True)
    # Small park canopies and moored boats establish the harbor scale.
    def tree(x,z,scale=1.):
        box(x,z,.035,.035,.27,.28*scale,(102,89,67))
        for row in range(5):
            a0=.10+row*(math.pi-.20)/5;a1=.10+(row+1)*(math.pi-.20)/5
            for side in range(8):
                b0=side*math.tau/8;b1=(side+1)*math.tau/8
                def point(a,b):return (x+.19*scale*math.sin(a)*math.cos(b),.5*scale+.26*scale*math.cos(a),z+.19*scale*math.sin(a)*math.sin(b))
                quad([point(a0,b0),point(a0,b1),point(a1,b1),point(a1,b0)],(70+row*4,113+row*5,67+row*3))
    for _ in range(100):tree(rng.uniform(4.4,9.3),rng.uniform(-.6,.6),rng.uniform(.7,1.1))
    for x in range(-13,15):
        for z in [-3.83,3.83]:tree(x,z,.72)
    for x,z in [(-11,4.8),(-3,-5.0),(8,5.1)]:
        box(x,z,.95,.28,-.36,.14,(216,220,202));box(x+.1,z,.4,.2,-.22,.12,(207,158,89))

    # Offline direct-light bake, as in a prelit city-building game asset.
    # Trace building silhouettes onto fine street/plaza tiles, not overlay decals.
    def blocked(x,z):
        for xmin,xmax,zmin,zmax,top in occluders:
            t0,t1=0.,max(0.,top-.26)
            for origin,direction,lo,hi in [(x,-.65,xmin,xmax),(z,.48,zmin,zmax)]:
                a,b=(lo-origin)/direction,(hi-origin)/direction
                t0=max(t0,min(a,b));t1=min(t1,max(a,b))
            if t0<t1:return True
        return False
    step=.18
    for ix in range(156):
        x=-13+ix*step
        for iz in range(46):
            z=-4.14+iz*step
            road=any(abs(z-lane)<.125 for lane in [-2.62,-1.385,0,1.385,2.62]) or abs((x+13.09+.59)%1.18-.59)<.09
            park=4.2<x<9.6 and abs(z)<.75
            rgb=(77,83,83) if road else ((86,122,77) if park else (159,158,143))
            if blocked(x,z):rgb=tuple(int(v*.64) for v in rgb)
            quad([(x-step/2,.252,z+step/2),(x+step/2,.252,z+step/2),(x+step/2,.252,z-step/2),(x-step/2,.252,z-step/2)],rgb)
    # Markings stay just above the ground, with the same depth ordering everywhere.
    for z in [-2.62,-1.385,0,1.385,2.62]:
        for x in range(-13,15):box(x,z,.3,.014,.254,.002,(191,183,143))

    blob=bytearray();views=[];accessors=[]
    def accessor(values,dims,fmt='f',component=5126):
        while len(blob)%4:blob.append(0)
        data=struct.pack('<'+fmt*len(values),*values)
        views.append(dict(buffer=0,byteOffset=len(blob),byteLength=len(data)));blob.extend(data)
        a=dict(bufferView=len(views)-1,componentType=component,count=len(values)//dims,type={1:'SCALAR',2:'VEC2',3:'VEC3',4:'VEC4'}[dims])
        if values is positions:a.update(min=[min(values[i::3]) for i in range(3)],max=[max(values[i::3]) for i in range(3)])
        accessors.append(a);return len(accessors)-1
    attributes=dict(POSITION=accessor(positions,3),NORMAL=accessor(normals,3),COLOR_0=accessor(colors,4),TEXCOORD_0=accessor(uvs,2))
    primitives=[dict(attributes=attributes,indices=accessor(plain_indices,1,'I',5125),material=0),dict(attributes=attributes,indices=accessor(glass_indices,1,'I',5125),material=1)]
    # Original baked facade texture. Fine mullions and subdued glass replace
    # random geometry color blocks; image is embedded in GLB, no runtime download.
    width,height=1536,1024
    rows=bytearray()
    for y in range(height):
        rows.append(0)
        for x in range(width):
            style=x//512
            cx,cy=(x%512)//16,y//16;px,py=x%16,y%16
            lit=((cx*37+cy*19)%53)==0
            if px<2 or py<2:color=(174,183,183)
            elif px==2 or py==2:color=(105,123,133)
            else:
                v=(cx*17+cy*7)%13
                color=(205+v,188+v,147+v) if lit else (132+v,158+v,171+v)
            if style==1:
                # Warm stone / brick, with recessed dark window openings.
                if px<4 or py<4: color=(196+(cy%3)*5,178+(cy%3)*5,148+(cy%3)*4)
                else: color=(117+v,133+v,135+v) if not lit else (211,191,146)
            elif style==2:
                if px<3 or py<3:color=(210,214,209)
                else:color=(144+v,161+v,167+v) if not lit else (218,202,163)
            rows.extend(color)
    def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',binascii.crc32(kind+data)&0xffffffff)
    png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(bytes(rows),9))+chunk(b'IEND',b'')
    while len(blob)%4:blob.append(0)
    image_view=len(views);views.append(dict(buffer=0,byteOffset=len(blob),byteLength=len(png)));blob.extend(png)
    document=dict(asset=dict(version='2.0',generator='TekesUI3D MIT architectural Manhattan diorama'),scene=0,scenes=[dict(nodes=[0])],
        nodes=[dict(name='Manhattan - artistic, not georeferenced',mesh=0)],meshes=[dict(name='City, harbor and detailed facades',primitives=primitives)],
        materials=[dict(pbrMetallicRoughness=dict(baseColorFactor=[1,1,1,1],metallicFactor=0,roughnessFactor=.8)),dict(pbrMetallicRoughness=dict(baseColorTexture=dict(index=0),metallicFactor=0,roughnessFactor=.5))],
        textures=[dict(source=0)],images=[dict(bufferView=image_view,mimeType='image/png')],
        buffers=[dict(byteLength=len(blob))],bufferViews=views,accessors=accessors)
    j=json.dumps(document,separators=(',',':')).encode();j+=b' '*((-len(j))%4)
    data=bytes(blob);data+=b'\0'*((-len(data))%4)
    glb=struct.pack('<III',0x46546c67,2,28+len(j)+len(data))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(data),0x004e4942)+data
    return glb,dict(buildings=count+4,vertices=len(positions)//3,triangles=(len(plain_indices)+len(glass_indices))//3,meshes=2)

if __name__=='__main__':
    glb,stats=build();out=Path(__file__).with_name('manhattan.glb');out.write_bytes(glb)
    print(out, len(glb), 'bytes', stats)
